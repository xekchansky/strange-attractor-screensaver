//! Points and rates in the attractor's three-dimensional phase space.

/// A single point in the 3D state space of an attractor's ODE system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct State {
    /// Position along the first axis of the attractor's phase space.
    pub x: f64,
    /// Position along the second axis.
    pub y: f64,
    /// Position along the third axis.
    pub z: f64,
}

impl State {
    /// Build a state from its three coordinates.
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

/// The instantaneous rates of change of the state variables — the right-hand
/// side of the ODE system evaluated at a given state and parameter set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Derivatives {
    /// Rate of change of [`State::x`].
    pub dx: f64,
    /// Rate of change of [`State::y`].
    pub dy: f64,
    /// Rate of change of [`State::z`].
    pub dz: f64,
}

impl Derivatives {
    /// Build the derivative triple.
    pub const fn new(dx: f64, dy: f64, dz: f64) -> Self {
        Self { dx, dy, dz }
    }
}
