//! The engine owns the state, steps it through the solver and keeps a bounded
//! trace.

mod support;

use std::collections::HashMap;

use attractor_core::{
    AttractorEngine, AttractorModel, DebugConfig, IntegrationMethod, State, catalog,
};
use support::lorenz_descriptor;

fn lorenz_model() -> AttractorModel {
    let parameters = HashMap::from([
        ("a".to_string(), 10.0),
        ("b".to_string(), 8.0 / 3.0),
        ("r".to_string(), 28.0),
    ]);
    AttractorModel::new(lorenz_descriptor(), parameters).unwrap()
}

fn make_engine(trail_capacity: usize) -> AttractorEngine {
    AttractorEngine::new(
        lorenz_model(),
        State::new(0.1, 0.1, 0.1),
        IntegrationMethod::RungeKutta4,
        0.01,
        trail_capacity,
    )
}

#[test]
fn starts_with_the_initial_state_as_the_sole_trace_point() {
    let engine = make_engine(50);
    assert_eq!(engine.trace().len(), 1);
    assert_eq!(Some(engine.state()), engine.trace().first());
}

#[test]
fn advancing_moves_the_state_and_grows_the_trace() {
    let mut engine = make_engine(2000);
    let initial = engine.state();
    engine.advance().unwrap();
    assert_ne!(engine.state(), initial);
    assert_eq!(engine.trace().len(), 2);
    assert_eq!(Some(engine.state()), engine.trace().last());
}

#[test]
fn trace_is_bounded_by_the_trail_capacity() {
    let mut engine = make_engine(50);
    for _ in 0..100 {
        engine.advance().unwrap();
    }
    assert_eq!(engine.trace().len(), 50);
    assert_eq!(Some(engine.state()), engine.trace().last());
}

#[test]
fn a_zero_capacity_is_clamped_to_one_point() {
    let engine = make_engine(0);
    assert_eq!(engine.trail_capacity, 1);
    assert_eq!(engine.trace().len(), 1);
}

#[test]
fn produced_trace_points_are_finite() {
    let mut engine = make_engine(100);
    for _ in 0..50 {
        engine.advance().unwrap();
    }
    for point in engine.trace().iter() {
        assert!(point.x.is_finite());
        assert!(point.y.is_finite());
        assert!(point.z.is_finite());
    }
}

#[test]
fn euler_and_rk4_engines_stay_finite_on_the_lorenz_system() {
    for method in [IntegrationMethod::Euler, IntegrationMethod::RungeKutta4] {
        let mut engine = AttractorEngine::new(
            lorenz_model(),
            State::new(0.1, 0.1, 0.1),
            method,
            0.004,
            500,
        );
        for _ in 0..500 {
            engine.advance().unwrap();
        }
        let state = engine.state();
        assert!(state.x.is_finite() && state.y.is_finite() && state.z.is_finite());
    }
}

#[test]
fn every_bundled_catalog_attractor_stays_finite_under_its_debug_defaults() {
    let catalog = catalog::bundled().unwrap();
    // Probe a deterministic spread of initial states: some otherwise lovely
    // flows have small basins of attraction and diverge from a particular
    // start, so a single debug-default start would hide that.
    let probe_starts = [
        State::new(0.01, -0.01, 0.01), // DebugConfig-like off-center
        State::new(0.5, -0.3, 0.2),
        State::new(0.9, 0.9, 0.9),
        State::new(-0.7, 0.4, -0.5),
    ];
    for descriptor in &catalog {
        let selection = DebugConfig::selection(&descriptor.id, &catalog).unwrap();
        let model = selection.build_model().unwrap(); // parses all formulas, incl. Aizawa's `^`
        for start in probe_starts {
            let mut engine = AttractorEngine::new(
                model.clone(),
                start,
                IntegrationMethod::RungeKutta4,
                selection.settings.step_size,
                2000,
            );
            for _ in 0..8000 {
                engine.advance().unwrap();
            }
            let state = engine.state();
            assert!(
                state.x.is_finite() && state.y.is_finite() && state.z.is_finite(),
                "{} diverged from {start:?}",
                descriptor.id
            );
        }
    }
}
