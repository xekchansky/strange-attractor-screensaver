//! Headless simulation session: deterministic frame stepping, health checks
//! and the geometry the renderer uploads.

use attractor_cli::{SessionError, SimulationSession};
use attractor_core::{AttractorSelection, DebugConfig, SplitMix64, State, catalog};
use attractor_view::TrailGeometry;

const FRAME_INTERVAL: f64 = 1.0 / 60.0;

fn lorenz() -> AttractorSelection {
    let catalog = catalog::bundled().expect("catalog decodes");
    DebugConfig::selection("lorenz", &catalog).expect("lorenz builds")
}

fn run_frames(seed: u64, warmup: usize, frames: usize) -> TrailGeometry {
    let mut rng = SplitMix64::new(seed);
    let mut session =
        SimulationSession::new(lorenz(), None, None, &mut rng).expect("session builds");
    session.warm_up(warmup).expect("warm-up stays healthy");
    for _ in 0..frames {
        session
            .advance_frame(FRAME_INTERVAL)
            .expect("frame advances");
    }
    session.geometry().clone()
}

#[test]
fn the_same_seed_produces_the_same_geometry() {
    let first = run_frames(7, 200, 5);
    let second = run_frames(7, 200, 5);
    assert_eq!(first.positions(), second.positions());
    assert_eq!(first.colors(), second.colors());
    assert_eq!(first.indices(), second.indices());
    assert!(!first.is_empty());
}

#[test]
fn different_seeds_produce_different_cameras() {
    // The camera azimuth is drawn from the same seeded source, so different
    // seeds must orbit to different sides and orient the ribbon differently.
    let first = run_frames(1, 50, 1);
    let second = run_frames(2, 50, 1);
    assert_ne!(first.positions(), second.positions());
}

#[test]
fn a_fixed_point_run_is_reported_as_unhealthy() {
    let mut selection = lorenz();
    // Lorenz's origin is an equilibrium: without a perturbation nothing moves.
    selection.initial_state = State::new(0.0, 0.0, 0.0);
    let mut rng = SplitMix64::new(1);
    let mut session =
        SimulationSession::new(selection, Some(0.0), Some(0.0), &mut rng).expect("session builds");
    match session.warm_up(1_500) {
        Err(SessionError::Unhealthy(issue)) => {
            assert_eq!(issue, attractor_core::SimulationHealthIssue::Collapsed)
        }
        other => panic!("expected a collapsed run, got {other:?}"),
    }
}

#[test]
fn frames_advance_the_simulation_and_refresh_the_geometry() {
    let mut rng = SplitMix64::new(3);
    let mut session =
        SimulationSession::new(lorenz(), Some(0.0), Some(0.0), &mut rng).expect("session builds");
    session.warm_up(100).expect("warm-up stays healthy");
    assert_eq!(session.frames_simulated(), 0);
    let before = session.geometry().vertex_count();
    session
        .advance_frame(FRAME_INTERVAL)
        .expect("frame advances");
    assert_eq!(session.frames_simulated(), 1);
    assert!(session.geometry().vertex_count() >= before);
}

#[test]
fn a_zero_frame_interval_still_rebuilds_the_frame() {
    let mut rng = SplitMix64::new(4);
    let mut session =
        SimulationSession::new(lorenz(), Some(0.0), Some(0.0), &mut rng).expect("session builds");
    session.warm_up(10).expect("warm-up stays healthy");
    let state = session.state();
    session.advance_frame(0.0).expect("a zero interval is safe");
    assert_eq!(session.frames_simulated(), 1);
    assert_eq!(session.state(), state);
}

#[test]
fn the_session_exposes_the_overlay_and_projection() {
    let mut rng = SplitMix64::new(5);
    let mut session =
        SimulationSession::new(lorenz(), Some(0.0), Some(0.0), &mut rng).expect("session builds");
    session.warm_up(10).expect("warm-up stays healthy");
    session
        .advance_frame(FRAME_INTERVAL)
        .expect("frame advances");

    let overlay = session.overlay();
    assert!(overlay.lines().len() >= 5);
    assert_eq!(overlay.lines()[0], "Lorenz");

    let projection = session.view_projection(4.0 / 3.0);
    let state = session.state();
    let clip = projection.project(attractor_view::Vector3::new(state.x, state.y, state.z));
    assert!(
        clip[3] > 0.0,
        "the current point must be in front of the camera"
    );
    let depth = clip[2] / clip[3];
    assert!((0.0..=1.0).contains(&depth), "clip depth {depth}");
}

#[test]
fn an_explicit_camera_angle_wins_over_the_seeded_draw() {
    let mut rng = SplitMix64::new(9);
    let session =
        SimulationSession::new(lorenz(), Some(0.0), Some(0.0), &mut rng).expect("session builds");
    assert_eq!(session.camera_azimuth(), 0.0);
    assert_eq!(session.camera_elevation(), 0.0);
}

#[test]
fn the_random_source_is_only_drawn_from_when_the_azimuth_is_unpinned() {
    let mut pinned = SplitMix64::new(11);
    let _ = SimulationSession::new(lorenz(), Some(0.0), Some(0.0), &mut pinned)
        .expect("session builds");
    let mut random = SplitMix64::new(11);
    let _ = SimulationSession::new(lorenz(), None, None, &mut random).expect("session builds");
    // `SplitMix64::new` stores the seed until the first draw.
    assert_eq!(pinned.state(), 11);
    assert_ne!(random.state(), pinned.state());
}
