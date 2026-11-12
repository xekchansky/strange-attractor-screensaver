//! Owns the live state of a simulation and advances it through the ODE solver.

use crate::formula::FormulaError;
use crate::ode_solver::{self, IntegrationMethod};
use crate::{AttractorModel, State, TraceBuffer};

/// Owns the state of a simulation, steps the system through numerical
/// integration, and keeps a bounded buffer of the most recent trace points.
#[derive(Debug, Clone, PartialEq)]
pub struct AttractorEngine {
    /// The integrable attractor model being simulated.
    pub model: AttractorModel,
    /// The integration scheme.
    pub method: IntegrationMethod,
    /// Integration time step (simulation accuracy).
    pub step_size: f64,
    /// Maximum number of points kept in [`Self::trace`].
    pub trail_capacity: usize,
    state: State,
    trace: TraceBuffer,
}

impl AttractorEngine {
    /// Create an engine with an initial state and a trace holding just that
    /// state.
    ///
    /// `trail_capacity` is clamped to at least one point.
    pub fn new(
        model: AttractorModel,
        initial_state: State,
        method: IntegrationMethod,
        step_size: f64,
        trail_capacity: usize,
    ) -> Self {
        let trail_capacity = trail_capacity.max(1);
        Self {
            model,
            method,
            step_size,
            trail_capacity,
            state: initial_state,
            trace: TraceBuffer::new(trail_capacity, initial_state),
        }
    }

    /// The current simulation state.
    pub const fn state(&self) -> State {
        self.state
    }

    /// The bounded ring buffer of the most recent states, oldest first.
    pub const fn trace(&self) -> &TraceBuffer {
        &self.trace
    }

    /// Advance the simulation by one integration step and record the new point.
    ///
    /// Returns the resulting state, or the [`FormulaError`] produced while
    /// evaluating the vector field (the state and the trace are left
    /// unchanged in that case).
    pub fn advance(&mut self) -> Result<State, FormulaError> {
        let next = ode_solver::step(self.state, self.step_size, self.method, |state| {
            self.model.rate(state)
        })?;
        self.state = next;
        self.trace.append(next);
        Ok(next)
    }
}
