//! The integration contract: Euler and RK4 arithmetic, accuracy and errors.

use attractor_core::formula::FormulaError;
use attractor_core::ode_solver::{self, IntegrationMethod};
use attractor_core::{Derivatives, State};

/// A vector field that is constant on the whole state space.
fn constant_field(_: State) -> Result<Derivatives, FormulaError> {
    Ok(Derivatives::new(2.0, -1.0, 0.5))
}

/// A diagonal flow `x' = -x`, `y' = 0`, `z' = 0` with the reference solution
/// `x(t) = exp(-t)`.
fn decay_field(state: State) -> Result<Derivatives, FormulaError> {
    Ok(Derivatives::new(-state.x, 0.0, 0.0))
}

/// A field that cannot be evaluated, to prove errors stop the step.
fn broken_field(_: State) -> Result<Derivatives, FormulaError> {
    Err(FormulaError::UnknownVariable("q".into()))
}

#[test]
fn euler_integrates_a_constant_field_exactly() {
    let after = ode_solver::euler(State::new(0.0, 0.0, 0.0), 0.1, constant_field)
        .expect("a constant field never fails");
    assert_eq!(after, State::new(0.2, -0.1, 0.05));
}

#[test]
fn rk4_integrates_a_constant_field_exactly() {
    let after = ode_solver::runge_kutta4(State::new(0.0, 0.0, 0.0), 0.1, constant_field)
        .expect("a constant field never fails");
    assert_eq!(after, State::new(0.2, -0.1, 0.05));
}

#[test]
fn rk4_is_more_accurate_than_euler_on_exponential_decay() {
    const STEPS: usize = 200;
    const DT: f64 = 0.01;

    let integrate = |method: IntegrationMethod| {
        let mut state = State::new(1.0, 0.0, 0.0);
        for _ in 0..STEPS {
            state = ode_solver::step(state, DT, method, decay_field).expect("decay never fails");
        }
        state.x
    };

    let exact = (-(STEPS as f64) * DT).exp();
    let euler = integrate(IntegrationMethod::Euler);
    let rk4 = integrate(IntegrationMethod::RungeKutta4);

    assert!(abs(euler - exact) > 1e-8);
    assert!(abs(rk4 - exact) < abs(euler - exact));
    assert!(abs(rk4 - exact) < 1e-5);
}

#[test]
fn rk4_error_scales_as_fourth_order_in_the_step_size() {
    let error = |step: f64| {
        let mut state = State::new(1.0, 0.0, 0.0);
        let steps = (1.0 / step) as usize;
        for _ in 0..steps {
            state = ode_solver::runge_kutta4(state, step, decay_field).expect("decay never fails");
        }
        abs(state.x - (-1.0f64).exp())
    };

    let coarse = error(0.1);
    let fine = error(0.05);
    let ratio = coarse / fine;
    // A fourth-order method roughly divides the error by 16 per halving of the
    // step, so the observed ratio should sit comfortably above 4.
    assert!((4.0..=80.0).contains(&ratio), "ratio was {ratio}");
}

#[test]
fn step_dispatches_to_the_chosen_method() {
    let state = State::new(0.0, 0.0, 0.0);
    let euler = ode_solver::step(state, 0.1, IntegrationMethod::Euler, constant_field).unwrap();
    let rk4 = ode_solver::step(state, 0.1, IntegrationMethod::RungeKutta4, constant_field).unwrap();
    assert_eq!(euler, rk4, "a constant field integrates exactly either way");
}

#[test]
fn a_failing_rate_stops_the_step_and_returns_its_error() {
    for method in [IntegrationMethod::Euler, IntegrationMethod::RungeKutta4] {
        let result = ode_solver::step(State::new(0.0, 0.0, 0.0), 0.1, method, broken_field);
        assert_eq!(
            result,
            Err(FormulaError::UnknownVariable("q".into())),
            "{method:?} must propagate the rate error"
        );

        let direct = match method {
            IntegrationMethod::Euler => {
                ode_solver::euler(State::new(0.0, 0.0, 0.0), 0.1, broken_field)
            }
            IntegrationMethod::RungeKutta4 => {
                ode_solver::runge_kutta4(State::new(0.0, 0.0, 0.0), 0.1, broken_field)
            }
        };
        assert_eq!(direct, Err(FormulaError::UnknownVariable("q".into())));
    }
}

fn abs(value: f64) -> f64 {
    value.abs()
}
