//! Detects simulations that have gone wrong: a frozen equilibrium, a stable
//! limit cycle, or a diverging trajectory.

use crate::State;

/// A detected problem with a running simulation.
///
/// The screensaver treats every issue the same way: it discards the current
/// selection and starts a completely new random simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationHealthIssue {
    /// The trajectory produced a NaN or an infinite coordinate.
    NonFinite,
    /// A coordinate grew past [`SimulationHealthThresholds::divergence_limit`].
    Diverged,
    /// The trajectory is confined to a tiny region: an equilibrium point or a
    /// very small limit cycle (Chua's near-zero ring).
    Collapsed,
    /// The trajectory returns to an earlier state: a stable limit cycle, large
    /// or small.
    Periodic,
}

/// Tuning knobs for [`SimulationHealthMonitor`].
///
/// Defaults are chosen for the bundled catalog: warm up for 1000 steps, then
/// check often for the first 10k steps (where bad launches show up), less
/// often up to 100k, and rarely afterwards. Cheap checks (non-finite and
/// divergence) still run on every state. The more expensive periodic check
/// runs every 512 states, and its verdict must persist for 4096 states before
/// the run is reported: chaotic flows can shadow an unstable orbit for a while.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimulationHealthThresholds {
    /// Number of observed states before structural checks (collapse/periodic)
    /// start. Prevents flagging a normal transient.
    pub warmup_steps: usize,
    /// Number of recent states used to measure the trajectory's spatial span.
    pub collapse_window_steps: usize,
    /// A span below this value means the trajectory is visually a point.
    pub collapse_span: f64,
    /// A coordinate magnitude above this value means the run has diverged.
    pub divergence_limit: f64,
    /// Maximum number of states remembered for recurrence checks.
    pub history_capacity: usize,
    /// Minimum lag (in steps) between the current state and an earlier one for
    /// that pair to count as a possible period.
    pub minimum_period_steps: usize,
    /// Number of recent states over which a candidate period is validated.
    pub periodic_validation_window: usize,
    /// Number of steps a periodic verdict must persist before it is reported.
    ///
    /// A chaotic flow can shadow an unstable periodic orbit for thousands of
    /// steps before wandering off. A genuine limit cycle keeps the verdict
    /// indefinitely, so requiring persistence separates the two.
    pub periodic_confirmation_steps: usize,
    /// How often the periodic check runs, in observed steps.
    ///
    /// The periodic check is much more expensive than the collapse check, so
    /// it runs less often than the structural check schedule would imply.
    pub periodic_check_interval: usize,
    /// Maximum RMS distance for a validated period, as a fraction of the
    /// recent trajectory's spatial span. Above this the motion is not a
    /// repeatable closed orbit.
    pub periodic_mean_distance_fraction: f64,
    /// Length of the hot phase, counted in observed states.
    pub hot_phase_steps: usize,
    /// Length of the warm phase, counted in observed states.
    pub warm_phase_steps: usize,
    /// Structural check interval during the hot phase.
    pub hot_check_interval: usize,
    /// Structural check interval during the warm phase.
    pub warm_check_interval: usize,
    /// Structural check interval after the warm phase.
    pub cold_check_interval: usize,
}

impl Default for SimulationHealthThresholds {
    fn default() -> Self {
        Self {
            warmup_steps: 1_000,
            collapse_window_steps: 1_000,
            collapse_span: 0.01,
            divergence_limit: 1e6,
            history_capacity: 4_096,
            minimum_period_steps: 64,
            periodic_validation_window: 2_048,
            periodic_confirmation_steps: 4_096,
            periodic_check_interval: 512,
            periodic_mean_distance_fraction: 0.1,
            hot_phase_steps: 10_000,
            warm_phase_steps: 100_000,
            hot_check_interval: 16,
            warm_check_interval: 128,
            cold_check_interval: 1_024,
        }
    }
}

impl SimulationHealthThresholds {
    /// Clamp every field into a range where the checks still make sense.
    ///
    /// A zero window would make the periodic test meaningless and a
    /// non-finite threshold would flip every comparison, so the monitor
    /// sanitizes its configuration on construction rather than trusting the
    /// caller. Negative values are impossible by type: the step counts are
    /// `usize`.
    pub fn sanitized(self) -> Self {
        let history_capacity = self.history_capacity.max(2);
        let hot_phase_steps = self.hot_phase_steps;
        Self {
            warmup_steps: self.warmup_steps,
            collapse_window_steps: self.collapse_window_steps.max(1),
            collapse_span: if self.collapse_span.is_finite() {
                self.collapse_span.max(0.0)
            } else {
                0.01
            },
            divergence_limit: if self.divergence_limit.is_finite() && self.divergence_limit > 0.0 {
                self.divergence_limit
            } else {
                1e6
            },
            history_capacity,
            minimum_period_steps: self.minimum_period_steps.max(1),
            periodic_validation_window: self
                .periodic_validation_window
                .clamp(1, history_capacity - 1),
            periodic_confirmation_steps: self.periodic_confirmation_steps.max(1),
            periodic_check_interval: self.periodic_check_interval.max(1),
            periodic_mean_distance_fraction: if self.periodic_mean_distance_fraction.is_finite() {
                self.periodic_mean_distance_fraction.clamp(0.0, 1.0)
            } else {
                0.1
            },
            hot_phase_steps,
            warm_phase_steps: self.warm_phase_steps.max(hot_phase_steps),
            hot_check_interval: self.hot_check_interval.max(1),
            warm_check_interval: self.warm_check_interval.max(1),
            cold_check_interval: self.cold_check_interval.max(1),
        }
    }
}

/// Watches a stream of simulation states and reports when the run has gone
/// wrong: it froze at an equilibrium, settled into a limit cycle, or blew up.
///
/// The monitor is pure and deterministic: it depends only on the observed
/// states and its configuration, so it is unit-testable without a renderer.
/// Feed it one state per integration step.
#[derive(Debug, Clone, PartialEq)]
pub struct SimulationHealthMonitor {
    thresholds: SimulationHealthThresholds,
    steps_observed: usize,
    check_count: usize,
    capacity: usize,
    history: Vec<State>,
    /// Next slot to overwrite once `history` is full; also the oldest entry.
    write_index: usize,
    saturated: bool,
    next_check_step: usize,
    /// When the current run of periodic verdicts started, if one is active.
    pending_periodic_since: Option<usize>,
    next_periodic_check_step: usize,
}

impl Default for SimulationHealthMonitor {
    fn default() -> Self {
        Self::new(SimulationHealthThresholds::default())
    }
}

impl SimulationHealthMonitor {
    /// Create a monitor with explicit thresholds, sanitized on construction.
    pub fn new(thresholds: SimulationHealthThresholds) -> Self {
        let thresholds = thresholds.sanitized();
        Self {
            capacity: thresholds.history_capacity,
            steps_observed: 0,
            check_count: 0,
            history: Vec::with_capacity(thresholds.history_capacity),
            write_index: 0,
            saturated: false,
            next_check_step: thresholds.warmup_steps,
            pending_periodic_since: None,
            next_periodic_check_step: thresholds.warmup_steps,
            thresholds,
        }
    }

    /// The configuration in effect.
    pub const fn thresholds(&self) -> &SimulationHealthThresholds {
        &self.thresholds
    }

    /// How many states have been observed so far.
    pub const fn steps_observed(&self) -> usize {
        self.steps_observed
    }

    /// How many full structural checks have run.
    ///
    /// Exposed so tests can verify the back-off schedule.
    pub const fn check_count(&self) -> usize {
        self.check_count
    }

    /// Observe the next state and return an issue, if the run has gone wrong.
    ///
    /// Non-finite and divergent states are reported immediately, on any step.
    /// Structural checks (collapse and periodicity) start after
    /// [`SimulationHealthThresholds::warmup_steps`] and are then performed on
    /// a schedule that gets sparser as the simulation ages. Collapse is
    /// reported from the first matching check; periodicity only after the
    /// verdict has persisted for
    /// [`SimulationHealthThresholds::periodic_confirmation_steps`].
    pub fn observe(&mut self, state: State) -> Option<SimulationHealthIssue> {
        if !is_finite(state) {
            return Some(SimulationHealthIssue::NonFinite);
        }
        if self.is_diverged(state) {
            return Some(SimulationHealthIssue::Diverged);
        }

        self.append(state);
        self.steps_observed += 1;

        if self.steps_observed < self.thresholds.warmup_steps
            || self.steps_observed < self.next_check_step
        {
            return None;
        }

        self.check_count += 1;
        self.next_check_step = self.steps_observed + self.check_interval_after(self.steps_observed);

        if self.is_collapsed() {
            return Some(SimulationHealthIssue::Collapsed);
        }
        if self.update_periodic_confirmation() {
            return Some(SimulationHealthIssue::Periodic);
        }
        None
    }

    /// Whether the recent trajectory is confined to a tiny region.
    fn is_collapsed(&self) -> bool {
        let window = self
            .thresholds
            .collapse_window_steps
            .clamp(1, self.history.len());
        if window <= 1 {
            return false;
        }
        self.recent_span(window) < self.thresholds.collapse_span
    }

    /// Track how long the trajectory has looked periodic and report it once
    /// the verdict has persisted long enough.
    ///
    /// One check is not conclusive: a chaotic flow can shadow an unstable
    /// periodic orbit for thousands of steps before wandering off. A genuine
    /// limit cycle reproduces the verdict on every check, so the pending
    /// timestamp survives until the run leaves the cycle.
    fn update_periodic_confirmation(&mut self) -> bool {
        if self.steps_observed < self.next_periodic_check_step {
            return false;
        }
        self.next_periodic_check_step =
            self.steps_observed + self.thresholds.periodic_check_interval.max(1);

        if !self.is_periodic() {
            self.pending_periodic_since = None;
            return false;
        }

        let since = self
            .pending_periodic_since
            .get_or_insert(self.steps_observed);
        self.steps_observed - *since >= self.thresholds.periodic_confirmation_steps.max(1)
    }

    /// Whether the recent trajectory keeps retracing a closed orbit.
    ///
    /// A limit cycle rarely has a period that is an integer number of
    /// integration steps, so no sampled state ever repeats exactly: the
    /// closest return is off by up to half a step along the orbit. Instead of
    /// demanding an exact match, find the lag at which the newest state
    /// returns closest, then check that *every* state in a recent window has a
    /// similarly close partner one lag earlier. A chaotic flow has close
    /// encounters too, but they do not persist across a few hundred states.
    fn is_periodic(&self) -> bool {
        let minimum_period = self.thresholds.minimum_period_steps.max(1);
        let validation_window = self
            .thresholds
            .periodic_validation_window
            .max(16)
            .min((self.history.len() / 2).max(1));
        let max_gap = match self.history.len().checked_sub(validation_window) {
            Some(max_gap) => max_gap,
            None => return false,
        };
        if max_gap < minimum_period {
            return false;
        }

        let scale_window = self
            .thresholds
            .collapse_window_steps
            .max(validation_window)
            .min(self.history.len());
        let scale = self.recent_span(scale_window);
        if scale <= 0.0 {
            return false;
        }

        // Keep the few lags whose newest state is closest to the current one.
        const CANDIDATE_LIMIT: usize = 16;
        let mut candidates: Vec<(usize, f64)> = Vec::with_capacity(CANDIDATE_LIMIT);
        let newest = self.recent_state(0);

        for gap in minimum_period..=max_gap {
            let distance = squared_distance(newest, self.recent_state(gap));
            if candidates.len() < CANDIDATE_LIMIT {
                candidates.push((gap, distance));
                candidates.sort_by(|lhs, rhs| lhs.1.total_cmp(&rhs.1));
            } else if distance < candidates[CANDIDATE_LIMIT - 1].1 {
                candidates[CANDIDATE_LIMIT - 1] = (gap, distance);
                candidates.sort_by(|lhs, rhs| lhs.1.total_cmp(&rhs.1));
            }
        }

        let threshold = self.thresholds.periodic_mean_distance_fraction * scale;
        let threshold_squared = threshold * threshold;
        for (gap, _) in candidates {
            let mean_squared = self.mean_squared_distance(gap, validation_window);
            if mean_squared > threshold_squared {
                continue;
            }

            // Ordinary smooth motion also keeps nearby states close for short
            // lags, and its chord distance grows sub-linearly, which can look
            // like a weak dip. A real period is a sharp dip: half a period
            // away the trajectory is on the opposite side of the loop, an
            // order of magnitude farther. For chaos no such lag exists.
            let half_gap = (gap / 2).max(1);
            let half_mean_squared = self.mean_squared_distance(half_gap, validation_window);
            if mean_squared * 100.0 <= half_mean_squared {
                return true;
            }
        }
        false
    }

    /// Mean squared distance between states one `gap` apart, over the last
    /// `window` states.
    fn mean_squared_distance(&self, gap: usize, window: usize) -> f64 {
        let mut sum = 0.0;
        for offset in 0..window {
            sum += squared_distance(self.recent_state(offset), self.recent_state(offset + gap));
        }
        sum / window as f64
    }

    /// The largest axis-aligned extent of the last `window` states.
    fn recent_span(&self, window: usize) -> f64 {
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut min_z = f64::INFINITY;
        let mut max_z = f64::NEG_INFINITY;

        for offset in 0..window {
            let state = self.recent_state(offset);
            min_x = min_x.min(state.x);
            max_x = max_x.max(state.x);
            min_y = min_y.min(state.y);
            max_y = max_y.max(state.y);
            min_z = min_z.min(state.z);
            max_z = max_z.max(state.z);
        }

        (max_x - min_x).max((max_y - min_y).max(max_z - min_z))
    }

    fn check_interval_after(&self, steps: usize) -> usize {
        if steps < self.thresholds.hot_phase_steps {
            self.thresholds.hot_check_interval.max(1)
        } else if steps < self.thresholds.warm_phase_steps {
            self.thresholds.warm_check_interval.max(1)
        } else {
            self.thresholds.cold_check_interval.max(1)
        }
    }

    fn is_diverged(&self, state: State) -> bool {
        let limit = self.thresholds.divergence_limit;
        state.x.abs() > limit || state.y.abs() > limit || state.z.abs() > limit
    }

    fn append(&mut self, state: State) {
        if self.history.len() < self.capacity {
            self.history.push(state);
            if self.history.len() == self.capacity {
                self.saturated = true;
                self.write_index = 0;
            }
        } else {
            self.history[self.write_index] = state;
            self.write_index = (self.write_index + 1) % self.capacity;
        }
    }

    /// The state `offset` steps back from the newest one (0 = newest).
    fn recent_state(&self, offset: usize) -> State {
        if self.saturated {
            let index = (self.write_index + 2 * self.capacity - 1 - offset) % self.capacity;
            self.history[index]
        } else {
            self.history[self.history.len() - 1 - offset]
        }
    }
}

fn is_finite(state: State) -> bool {
    state.x.is_finite() && state.y.is_finite() && state.z.is_finite()
}

fn squared_distance(lhs: State, rhs: State) -> f64 {
    let dx = lhs.x - rhs.x;
    let dy = lhs.y - rhs.y;
    let dz = lhs.z - rhs.z;
    dx * dx + dy * dy + dz * dz
}
