//! Numerical integration of an attractor's three-dimensional ODE system.

use crate::formula::FormulaError;
use crate::{Derivatives, State};

/// The numerical integration scheme used to advance the ODE system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationMethod {
    /// First-order explicit Euler method.
    Euler,
    /// Fourth-order explicit Runge–Kutta method (RK4).
    RungeKutta4,
}

/// Advance `state` by `dt` using `method`.
///
/// `rate` is the vector field of the system; returning an error stops the step
/// and propagates that error unchanged. The arithmetic mirrors the Swift
/// oracle exactly, so seeded trajectories stay within the golden tolerance.
pub fn step<F>(
    state: State,
    dt: f64,
    method: IntegrationMethod,
    rate: F,
) -> Result<State, FormulaError>
where
    F: Fn(State) -> Result<Derivatives, FormulaError>,
{
    match method {
        IntegrationMethod::Euler => euler(state, dt, rate),
        IntegrationMethod::RungeKutta4 => runge_kutta4(state, dt, rate),
    }
}

/// First-order explicit Euler step: `s' = s + h * f(s)`.
pub fn euler<F>(state: State, dt: f64, rate: F) -> Result<State, FormulaError>
where
    F: Fn(State) -> Result<Derivatives, FormulaError>,
{
    let k = rate(state)?;
    Ok(State::new(
        state.x + dt * k.dx,
        state.y + dt * k.dy,
        state.z + dt * k.dz,
    ))
}

/// Classic fourth-order Runge–Kutta step.
pub fn runge_kutta4<F>(state: State, dt: f64, rate: F) -> Result<State, FormulaError>
where
    F: Fn(State) -> Result<Derivatives, FormulaError>,
{
    let k1 = rate(state)?;

    let s2 = State::new(
        state.x + 0.5 * dt * k1.dx,
        state.y + 0.5 * dt * k1.dy,
        state.z + 0.5 * dt * k1.dz,
    );
    let k2 = rate(s2)?;

    let s3 = State::new(
        state.x + 0.5 * dt * k2.dx,
        state.y + 0.5 * dt * k2.dy,
        state.z + 0.5 * dt * k2.dz,
    );
    let k3 = rate(s3)?;

    let s4 = State::new(
        state.x + dt * k3.dx,
        state.y + dt * k3.dy,
        state.z + dt * k3.dz,
    );
    let k4 = rate(s4)?;

    Ok(State::new(
        state.x + dt / 6.0 * (k1.dx + 2.0 * k2.dx + 2.0 * k3.dx + k4.dx),
        state.y + dt / 6.0 * (k1.dy + 2.0 * k2.dy + 2.0 * k3.dy + k4.dy),
        state.z + dt / 6.0 * (k1.dz + 2.0 * k2.dz + 2.0 * k3.dz + k4.dz),
    ))
}
