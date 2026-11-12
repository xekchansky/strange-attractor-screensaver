//! Turns wall-clock frame intervals into whole ODE integration steps.

const DEFAULT_MAXIMUM_FRAME_INTERVAL: f64 = 0.1;
const DEFAULT_MAXIMUM_STEPS_PER_FRAME: usize = 4_096;
const HARD_STEP_CAP: usize = 1_000_000;

/// Turns wall-clock frame intervals into whole ODE integration steps.
///
/// The renderer runs on a timer, and a monotonic clock jumps by the whole
/// sleep/suspend duration after the machine wakes up. Feeding that interval
/// straight into `simulated time / step size` would request tens of thousands
/// of integration steps in one frame, and a zero step size would divide by
/// zero. This scheduler bounds both the accepted frame interval and the number
/// of steps per frame, and drops the accumulated backlog once the cap is hit,
/// so a stall costs one slow frame instead of a freeze.
#[derive(Debug, Clone, PartialEq)]
pub struct SimulationStepScheduler {
    step_size: f64,
    simulation_speed: f64,
    maximum_frame_interval: f64,
    maximum_steps_per_frame: usize,
    accumulated_time: f64,
}

impl SimulationStepScheduler {
    /// Create a scheduler for one run with the default limits.
    ///
    /// `step_size` must be positive to produce steps; a non-positive
    /// `simulation_speed` stops stepping.
    pub fn new(step_size: f64, simulation_speed: f64) -> Self {
        Self::with_limits(
            step_size,
            simulation_speed,
            DEFAULT_MAXIMUM_FRAME_INTERVAL,
            DEFAULT_MAXIMUM_STEPS_PER_FRAME,
        )
    }

    /// Create a scheduler with explicit limits.
    ///
    /// `maximum_frame_interval` must be finite and positive, otherwise 0.1 s
    /// is used; `maximum_steps_per_frame` is clamped to `1..=1_000_000`.
    pub fn with_limits(
        step_size: f64,
        simulation_speed: f64,
        maximum_frame_interval: f64,
        maximum_steps_per_frame: usize,
    ) -> Self {
        Self {
            step_size,
            simulation_speed,
            maximum_frame_interval: if maximum_frame_interval.is_finite()
                && maximum_frame_interval > 0.0
            {
                maximum_frame_interval
            } else {
                DEFAULT_MAXIMUM_FRAME_INTERVAL
            },
            maximum_steps_per_frame: maximum_steps_per_frame.clamp(1, HARD_STEP_CAP),
            accumulated_time: 0.0,
        }
    }

    /// Integration step size in simulated-time units.
    pub const fn step_size(&self) -> f64 {
        self.step_size
    }

    /// Change the integration step size.
    pub const fn set_step_size(&mut self, step_size: f64) {
        self.step_size = step_size;
    }

    /// Multiplier applied to wall-clock time.
    pub const fn simulation_speed(&self) -> f64 {
        self.simulation_speed
    }

    /// Change the wall-clock time multiplier.
    pub const fn set_simulation_speed(&mut self, simulation_speed: f64) {
        self.simulation_speed = simulation_speed;
    }

    /// Longest frame interval the scheduler accounts for.
    pub const fn maximum_frame_interval(&self) -> f64 {
        self.maximum_frame_interval
    }

    /// Hard cap on the number of integration steps performed in one call.
    pub const fn maximum_steps_per_frame(&self) -> usize {
        self.maximum_steps_per_frame
    }

    /// Simulated time accumulated from previous frames but not consumed yet.
    pub const fn accumulated_time(&self) -> f64 {
        self.accumulated_time
    }

    /// Consume one rendered frame and return how many integration steps to run.
    ///
    /// Non-finite or non-positive inputs yield zero steps rather than a crash.
    /// If the requested work exceeds [`Self::maximum_steps_per_frame`], the
    /// backlog is discarded: replaying minutes of missed simulation after a
    /// sleep/wake would keep the next frames slow for a long time.
    pub fn steps_for_frame_interval(&mut self, frame_interval: f64) -> usize {
        if !(frame_interval.is_finite()
            && frame_interval > 0.0
            && self.step_size.is_finite()
            && self.step_size > 0.0
            && self.simulation_speed.is_finite()
            && self.simulation_speed > 0.0)
        {
            return 0;
        }

        self.accumulated_time +=
            self.simulation_speed * frame_interval.min(self.maximum_frame_interval);

        let requested = self.accumulated_time / self.step_size;
        if !requested.is_finite() {
            self.accumulated_time = 0.0;
            return 0;
        }

        if requested >= self.maximum_steps_per_frame as f64 {
            self.accumulated_time = 0.0;
            return self.maximum_steps_per_frame;
        }

        let steps = requested as usize;
        self.accumulated_time -= steps as f64 * self.step_size;
        steps
    }
}
