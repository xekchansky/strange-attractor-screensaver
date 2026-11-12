//! Limits how often a failed simulation may be replaced within a short window.

const DEFAULT_MAXIMUM_RESTARTS: usize = 3;
const DEFAULT_WINDOW: f64 = 10.0;

/// Limits how often a failed simulation may be replaced within a short window.
///
/// A run that fails immediately (for example, a formula that cannot be
/// evaluated) would otherwise be restarted on every frame, producing a flicker
/// and a CPU storm. The limiter allows a small burst of restarts, then asks the
/// caller to wait until the oldest one leaves the window.
#[derive(Debug, Clone, PartialEq)]
pub struct RestartLimiter {
    maximum_restarts: usize,
    window: f64,
    timestamps: Vec<f64>,
}

impl RestartLimiter {
    /// Create a limiter.
    ///
    /// `maximum_restarts` below 1 is raised to 1; a non-positive or non-finite
    /// `window` falls back to 10 seconds.
    pub fn new(maximum_restarts: usize, window: f64) -> Self {
        Self {
            maximum_restarts: maximum_restarts.max(1),
            window: if window.is_finite() && window > 0.0 {
                window
            } else {
                DEFAULT_WINDOW
            },
            timestamps: Vec::new(),
        }
    }

    /// Restart budget per window.
    pub const fn maximum_restarts(&self) -> usize {
        self.maximum_restarts
    }

    /// Sliding window length, in seconds.
    pub const fn window(&self) -> f64 {
        self.window
    }

    /// Record a restart at `now` if the limit allows it.
    ///
    /// Returns `true` when the restart may proceed and `false` when the caller
    /// should wait until [`Self::next_retry_time`].
    pub fn allow_restart(&mut self, now: f64) -> bool {
        self.discard_expired(now);
        if self.timestamps.len() < self.maximum_restarts {
            self.timestamps.push(now);
            true
        } else {
            false
        }
    }

    /// Earliest time a blocked restart may be retried.
    ///
    /// Returns `None` when a restart is allowed right now, otherwise the moment
    /// the oldest recorded restart leaves the window.
    pub fn next_retry_time(&self, now: f64) -> Option<f64> {
        let mut candidate = self.clone();
        candidate.discard_expired(now);
        if candidate.timestamps.len() >= self.maximum_restarts {
            candidate
                .timestamps
                .first()
                .map(|oldest| oldest + self.window)
        } else {
            None
        }
    }

    /// Forget every recorded restart (e.g. when a new animation session
    /// starts).
    pub fn reset(&mut self) {
        self.timestamps.clear();
    }

    fn discard_expired(&mut self, now: f64) {
        // Keep every stamp that has not left the window. An incomparable
        // (NaN) age also stays recorded, matching the Swift oracle, whose
        // `now - $0 >= window` predicate is false for NaN.
        self.timestamps.retain(|timestamp| {
            !matches!(
                (now - timestamp).partial_cmp(&self.window),
                Some(std::cmp::Ordering::Equal | std::cmp::Ordering::Greater)
            )
        });
    }
}

impl Default for RestartLimiter {
    fn default() -> Self {
        Self::new(DEFAULT_MAXIMUM_RESTARTS, DEFAULT_WINDOW)
    }
}
