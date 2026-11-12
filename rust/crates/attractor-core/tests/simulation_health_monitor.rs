//! The health monitor tells healthy chaos apart from degenerate runs: fixed
//! points, tiny rings, stable limit cycles and diverging trajectories.

mod support;

use std::collections::HashMap;
use std::f64::consts::PI;

use attractor_core::{
    AttractorEngine, AttractorModel, DebugConfig, IntegrationMethod, SimulationHealthIssue,
    SimulationHealthMonitor, SimulationHealthThresholds, State, catalog,
};
use support::lorenz_descriptor;

fn lorenz_engine(initial_state: State, step_size: f64) -> AttractorEngine {
    let parameters = HashMap::from([
        ("a".to_string(), 10.0),
        ("b".to_string(), 8.0 / 3.0),
        ("r".to_string(), 28.0),
    ]);
    let model = AttractorModel::new(lorenz_descriptor(), parameters).unwrap();
    AttractorEngine::new(
        model,
        initial_state,
        IntegrationMethod::RungeKutta4,
        step_size,
        100,
    )
}

/// A healthy chaotic trajectory must not be mistaken for a failure.
#[test]
fn a_chaotic_lorenz_trajectory_stays_healthy() {
    let mut engine = lorenz_engine(State::new(0.5, -0.3, 0.2), 0.004);
    let mut monitor = SimulationHealthMonitor::default();
    assert_eq!(monitor.observe(engine.state()), None);

    for _ in 0..20_000 {
        engine.advance().unwrap();
        assert_eq!(monitor.observe(engine.state()), None);
    }
}

/// Aizawa has long laminar phases that shadow an unstable periodic orbit for
/// about two thousand steps before the trajectory wanders off. The
/// confirmation interval must reject them; treating one matching window as
/// proof would heal a healthy run.
#[test]
fn aizawas_laminar_phases_are_not_mistaken_for_a_limit_cycle() {
    let catalog = catalog::bundled().unwrap();
    let selection = DebugConfig::selection("aizawa", &catalog).unwrap();
    let mut engine = AttractorEngine::new(
        selection.build_model().unwrap(),
        selection.initial_state,
        IntegrationMethod::RungeKutta4,
        selection.settings.step_size,
        100,
    );
    let mut monitor = SimulationHealthMonitor::default();

    for _ in 0..16_384 {
        engine.advance().unwrap();
        let issue = monitor.observe(engine.state());
        assert_eq!(issue, None, "aizawa: {issue:?}");
        if issue.is_some() {
            break;
        }
    }
}

/// A resting point is the simplest degenerate launch.
#[test]
fn a_fixed_point_is_reported_as_collapsed() {
    let mut monitor = SimulationHealthMonitor::default();
    let point = State::new(0.1, 0.2, 0.3);

    let mut issue = None;
    for _ in 0..10_000 {
        if issue.is_some() {
            break;
        }
        issue = monitor.observe(point);
    }
    assert_eq!(issue, Some(SimulationHealthIssue::Collapsed));
}

/// Chua's near-zero ring is a tiny periodic orbit: the span criterion catches
/// it before the recurrence criterion would.
#[test]
fn a_tiny_limit_cycle_is_reported_as_collapsed() {
    let mut monitor = SimulationHealthMonitor::new(SimulationHealthThresholds {
        warmup_steps: 100,
        ..Default::default()
    });

    let mut issue = None;
    for step in 0..5_000 {
        if issue.is_some() {
            break;
        }
        let angle = 2.0 * PI * (step % 97) as f64 / 97.0;
        issue = monitor.observe(State::new(0.001 * angle.cos(), 0.001 * angle.sin(), 0.0));
    }
    assert_eq!(issue, Some(SimulationHealthIssue::Collapsed));
}

/// A large stable limit cycle moves visibly but is not chaotic.
#[test]
fn a_large_limit_cycle_is_reported_as_periodic() {
    let mut monitor = SimulationHealthMonitor::new(SimulationHealthThresholds {
        warmup_steps: 100,
        minimum_period_steps: 64,
        ..Default::default()
    });

    let mut issue = None;
    for step in 0..5_000 {
        if issue.is_some() {
            break;
        }
        let angle = 2.0 * PI * (step % 200) as f64 / 200.0;
        issue = monitor.observe(State::new(5.0 * angle.cos(), 5.0 * angle.sin(), 0.0));
    }
    assert_eq!(issue, Some(SimulationHealthIssue::Periodic));
}

/// A chaotic flow can look periodic for a while, and a single matching window
/// is not enough to declare a limit cycle. The verdict must survive the whole
/// confirmation interval; an interruption restarts the count.
#[test]
fn a_periodic_verdict_must_persist_before_it_is_reported() {
    fn verdict(break_start: Option<usize>) -> Option<SimulationHealthIssue> {
        let thresholds = SimulationHealthThresholds {
            warmup_steps: 100,
            minimum_period_steps: 64,
            periodic_validation_window: 128,
            periodic_confirmation_steps: 2_000,
            periodic_check_interval: 50,
            hot_check_interval: 1,
            ..Default::default()
        };
        let mut monitor = SimulationHealthMonitor::new(thresholds);

        for step in 0..4_000 {
            if let Some(start) = break_start
                && step >= start
                && step < start + 500
            {
                let wandering = State::new(
                    0.01 * step as f64,
                    (0.3 * step as f64).sin(),
                    (0.17 * step as f64).cos(),
                );
                if let Some(issue) = monitor.observe(wandering) {
                    return Some(issue);
                }
                continue;
            }
            let angle = 2.0 * PI * (step % 200) as f64 / 200.0;
            if let Some(issue) =
                monitor.observe(State::new(5.0 * angle.cos(), 5.0 * angle.sin(), 0.0))
            {
                return Some(issue);
            }
        }
        None
    }

    assert_eq!(verdict(None), Some(SimulationHealthIssue::Periodic));
    assert_eq!(verdict(Some(2_000)), None);
}

#[test]
fn a_non_finite_state_is_reported_immediately() {
    let mut monitor = SimulationHealthMonitor::default();
    assert_eq!(
        monitor.observe(State::new(f64::NAN, 0.0, 0.0)),
        Some(SimulationHealthIssue::NonFinite)
    );
    assert_eq!(
        monitor.observe(State::new(0.0, f64::INFINITY, 0.0)),
        Some(SimulationHealthIssue::NonFinite)
    );
}

#[test]
fn a_divergent_state_is_reported_immediately() {
    let mut monitor = SimulationHealthMonitor::default();
    assert_eq!(
        monitor.observe(State::new(1e7, 0.0, 0.0)),
        Some(SimulationHealthIssue::Diverged)
    );
}

#[test]
fn structural_checks_do_not_run_before_the_warmup_ends() {
    let mut monitor = SimulationHealthMonitor::new(SimulationHealthThresholds {
        warmup_steps: 500,
        ..Default::default()
    });
    let point = State::new(0.1, 0.2, 0.3);

    for _ in 0..499 {
        assert_eq!(monitor.observe(point), None);
    }
    assert_eq!(monitor.steps_observed(), 499);
    assert_eq!(
        monitor.observe(point),
        Some(SimulationHealthIssue::Collapsed)
    );
    assert_eq!(monitor.steps_observed(), 500);
}

/// Checks must get rarer as the run ages: a healthy 10k-step run under the
/// three-phase schedule performs far fewer full checks than the hot interval
/// alone would imply.
#[test]
fn structural_checks_back_off_as_the_run_ages() {
    let thresholds = SimulationHealthThresholds {
        warmup_steps: 50,
        hot_phase_steps: 200,
        warm_phase_steps: 1_000,
        hot_check_interval: 5,
        warm_check_interval: 20,
        cold_check_interval: 100,
        ..Default::default()
    };
    let mut monitor = SimulationHealthMonitor::new(thresholds);

    for step in 0..10_000 {
        let state = State::new(
            0.001 * step as f64,
            (step as f64 * 0.1).sin(),
            (step as f64 * 0.07).cos(),
        );
        assert_eq!(monitor.observe(state), None);
    }
    // Every-5-steps throughout would be ~2000 checks; the schedule must stay
    // well below that.
    assert!(monitor.check_count() < 500);
    assert!(monitor.check_count() > 100);
}

/// A cycle that appears only after a long healthy run is still detected,
/// within one cold-phase interval of the window refilling.
#[test]
fn a_late_collapse_is_still_detected_after_checks_back_off() {
    let thresholds = SimulationHealthThresholds {
        warmup_steps: 50,
        collapse_window_steps: 100,
        hot_phase_steps: 500,
        warm_phase_steps: 1_000,
        hot_check_interval: 5,
        warm_check_interval: 20,
        cold_check_interval: 50,
        ..Default::default()
    };
    let mut monitor = SimulationHealthMonitor::new(thresholds);

    for step in 0..5_000 {
        let state = State::new(
            0.001 * step as f64,
            (step as f64 * 0.1).sin(),
            (step as f64 * 0.07).cos(),
        );
        assert_eq!(monitor.observe(state), None);
    }

    let fixed = State::new(0.1, 0.2, 0.3);
    let mut issue = None;
    let mut detected_after = 0;
    for offset in 1..=500 {
        if issue.is_some() {
            break;
        }
        issue = monitor.observe(fixed);
        detected_after = offset;
    }
    assert_eq!(issue, Some(SimulationHealthIssue::Collapsed));
    let limits = monitor.thresholds();
    assert!(
        detected_after <= limits.collapse_window_steps + limits.cold_check_interval,
        "detected after {detected_after} states"
    );
}

/// Calibration guard for the motivating case: Chua develops its double scroll
/// spontaneously from a near-zero start, and that slow start must not be
/// mistaken for the near-zero ring the monitor is meant to heal. The debug
/// midpoint parameters settle into a large cycle much later, which is a
/// genuine failure; only the slow start is checked here.
///
/// See also `the_reported_chua_limit_cycle_is_reported_as_periodic` below.
#[test]
fn chuas_slow_start_is_not_flagged() {
    let catalog = catalog::bundled().unwrap();
    let selection = DebugConfig::selection("chua", &catalog).unwrap();
    let mut engine = AttractorEngine::new(
        selection.build_model().unwrap(),
        selection.initial_state,
        IntegrationMethod::RungeKutta4,
        selection.settings.step_size,
        100,
    );
    let mut monitor = SimulationHealthMonitor::default();
    assert_eq!(monitor.observe(engine.state()), None);

    for _ in 0..5_000 {
        engine.advance().unwrap();
        let issue = monitor.observe(engine.state());
        assert_eq!(issue, None, "chua: {issue:?}");
        if issue.is_some() {
            break;
        }
    }
}

/// A real sampled limit cycle usually has a period that is not an integer
/// number of integration steps, so no pair of samples ever repeats exactly.
/// The monitor must still recognise the closed orbit.
#[test]
fn a_sampled_limit_cycle_with_a_non_integer_period_is_reported_as_periodic() {
    let mut monitor = SimulationHealthMonitor::new(SimulationHealthThresholds {
        warmup_steps: 100,
        ..Default::default()
    });
    let period = 200.5;

    let mut issue = None;
    for step in 0..20_000 {
        if issue.is_some() {
            break;
        }
        let angle = 2.0 * PI * step as f64 / period;
        issue = monitor.observe(State::new(5.0 * angle.cos(), 5.0 * angle.sin(), 0.0));
    }
    assert_eq!(issue, Some(SimulationHealthIssue::Periodic));
}

/// Regression for the reported Chua run: these parameters settle into a large
/// ring (a stable limit cycle, period ≈ 431.2 steps at dt=0.004) that the
/// exact-recurrence detector could never see.
#[test]
fn the_reported_chua_limit_cycle_is_reported_as_periodic() {
    let catalog = catalog::bundled().unwrap();
    let descriptor = catalog
        .into_iter()
        .find(|descriptor| descriptor.id == "chua")
        .unwrap();
    let parameters = HashMap::from([
        ("alpha".to_string(), 10.164),
        ("beta".to_string(), 14.932),
        ("m0".to_string(), -0.499),
        ("m1".to_string(), 0.257),
    ]);
    let model = AttractorModel::new(descriptor, parameters).unwrap();
    let mut engine = AttractorEngine::new(
        model,
        State::new(-0.243, 0.468, 0.093),
        IntegrationMethod::RungeKutta4,
        0.004,
        100,
    );
    let mut monitor = SimulationHealthMonitor::default();

    let mut issue = None;
    for _ in 0..60_000 {
        if issue.is_some() {
            break;
        }
        engine.advance().unwrap();
        issue = monitor.observe(engine.state());
    }
    assert_eq!(issue, Some(SimulationHealthIssue::Periodic));
}

#[test]
fn invalid_thresholds_are_sanitized_instead_of_flipping_every_check() {
    let thresholds = SimulationHealthThresholds {
        warmup_steps: 0,
        collapse_window_steps: 0,
        collapse_span: f64::NAN,
        divergence_limit: -1.0,
        history_capacity: 0,
        minimum_period_steps: 0,
        periodic_validation_window: 10_000,
        periodic_check_interval: 0,
        periodic_mean_distance_fraction: 5.0,
        hot_phase_steps: 0,
        warm_phase_steps: 0,
        hot_check_interval: 0,
        warm_check_interval: 0,
        cold_check_interval: 0,
        periodic_confirmation_steps: 0,
    }
    .sanitized();

    assert!(thresholds.collapse_window_steps >= 1);
    assert!(thresholds.collapse_span.is_finite() && thresholds.collapse_span >= 0.0);
    assert!(thresholds.divergence_limit > 0.0);
    assert!(thresholds.history_capacity >= 2);
    assert!(thresholds.minimum_period_steps >= 1);
    assert!(thresholds.periodic_validation_window >= 1);
    assert!(thresholds.periodic_validation_window < thresholds.history_capacity);
    assert!(thresholds.periodic_check_interval >= 1);
    assert!(thresholds.periodic_mean_distance_fraction >= 0.0);
    assert!(thresholds.periodic_mean_distance_fraction <= 1.0);
    assert!(thresholds.warm_phase_steps >= thresholds.hot_phase_steps);
    assert!(thresholds.hot_check_interval >= 1);
    assert!(thresholds.warm_check_interval >= 1);
    assert!(thresholds.cold_check_interval >= 1);
}
