//! Deterministic stepping, healing, resizing and rendering of one player.

use attractor_core::{
    AttractorDescriptor, AttractorSelection, DebugConfig, SplitMix64, State, catalog,
};
use attractor_gpu::{GpuContext, lit_pixels};
use attractor_player::{Player, PlayerError, PlayerOptions, RestartOutcome, TickEvent};
use attractor_view::{TrailGeometry, Vector3};

const FRAME_INTERVAL: f64 = 1.0 / 60.0;

fn validated_catalog() -> Vec<AttractorDescriptor> {
    catalog::bundled_validation()
        .expect("catalog decodes")
        .valid
}

fn lorenz() -> AttractorSelection {
    let catalog = catalog::bundled().expect("catalog decodes");
    DebugConfig::selection("lorenz", &catalog).expect("lorenz builds")
}

fn pinned_options() -> PlayerOptions {
    PlayerOptions {
        camera_azimuth: Some(0.0),
        camera_elevation: Some(0.0),
        heal_automatically: false,
        width: 320,
        height: 240,
        overlay_scale: 1.0,
        ..PlayerOptions::default()
    }
}

fn player(seed: u64, options: PlayerOptions) -> Player {
    Player::new(
        lorenz(),
        validated_catalog(),
        options,
        SplitMix64::new(seed),
    )
    .expect("the player builds")
}

fn free_camera_options() -> PlayerOptions {
    PlayerOptions {
        heal_automatically: false,
        width: 320,
        height: 240,
        overlay_scale: 1.0,
        ..PlayerOptions::default()
    }
}

fn run_frames(seed: u64, warmup: usize, frames: usize, options: PlayerOptions) -> TrailGeometry {
    let mut player = player(seed, options);
    player.warm_up(warmup).expect("warm-up stays healthy");
    for _ in 0..frames {
        player.tick(FRAME_INTERVAL).expect("frame advances");
    }
    player.geometry().clone()
}

#[test]
fn the_same_seed_produces_the_same_geometry() {
    let first = run_frames(7, 200, 5, pinned_options());
    let second = run_frames(7, 200, 5, pinned_options());
    assert_eq!(first.positions(), second.positions());
    assert_eq!(first.colors(), second.colors());
    assert_eq!(first.indices(), second.indices());
    assert!(!first.is_empty());
}

#[test]
fn different_seeds_produce_different_cameras() {
    // The camera azimuth is drawn from the same seeded source, so different
    // seeds must orbit to different sides and orient the ribbon differently.
    let first = run_frames(1, 50, 1, free_camera_options());
    let second = run_frames(2, 50, 1, free_camera_options());
    assert_ne!(first.positions(), second.positions());
}

#[test]
fn an_explicit_camera_angle_wins_over_the_seeded_draw() {
    let player = player(9, pinned_options());
    assert_eq!(player.camera_azimuth(), 0.0);
    assert_eq!(player.camera_elevation(), 0.0);
}

#[test]
fn frames_advance_the_simulation_and_refresh_the_geometry() {
    let mut player = player(3, pinned_options());
    player.warm_up(100).expect("warm-up stays healthy");
    assert_eq!(player.frames_simulated(), 0);
    let before = player.geometry().vertex_count();
    let event = player.tick(FRAME_INTERVAL).expect("frame advances");
    assert_eq!(event, TickEvent::Advanced);
    assert_eq!(player.frames_simulated(), 1);
    assert!(player.geometry().vertex_count() >= before);
}

#[test]
fn a_zero_frame_interval_still_rebuilds_the_frame() {
    let mut player = player(4, pinned_options());
    player.warm_up(10).expect("warm-up stays healthy");
    let state = player.state();
    player.tick(0.0).expect("a zero interval is safe");
    assert_eq!(player.frames_simulated(), 1);
    assert_eq!(player.state(), state);
}

#[test]
fn a_non_finite_frame_interval_refreshes_without_stepping() {
    let mut player = player(6, pinned_options());
    player.warm_up(10).expect("warm-up stays healthy");
    let state = player.state();
    for interval in [f64::NAN, f64::NEG_INFINITY, -1.0] {
        player.tick(interval).expect("a bad interval is ignored");
    }
    assert_eq!(player.frames_simulated(), 3);
    assert_eq!(player.state(), state);
    assert!(player.camera_azimuth().is_finite());
    assert_eq!(player.elapsed_seconds(), 0.0);
}

#[test]
fn the_player_exposes_the_overlay_and_projection() {
    let mut player = player(5, pinned_options());
    player.warm_up(10).expect("warm-up stays healthy");
    player.tick(FRAME_INTERVAL).expect("frame advances");

    let overlay = player.overlay();
    assert!(overlay.lines().len() >= 5);
    assert_eq!(overlay.lines()[0], "Lorenz");

    let projection = player.view_projection(4.0 / 3.0);
    let state = player.state();
    let clip = projection.project(Vector3::new(state.x, state.y, state.z));
    assert!(
        clip[3] > 0.0,
        "the current point must be in front of the camera"
    );
    let depth = clip[2] / clip[3];
    assert!((0.0..=1.0).contains(&depth), "clip depth {depth}");
}

#[test]
fn a_fixed_point_run_is_reported_when_healing_is_off() {
    let mut selection = lorenz();
    // Lorenz's origin is an equilibrium: without a perturbation nothing moves.
    selection.initial_state = State::new(0.0, 0.0, 0.0);
    let mut player = Player::new(
        selection,
        validated_catalog(),
        pinned_options(),
        SplitMix64::new(1),
    )
    .expect("the player builds");
    match player.warm_up(1_500) {
        Err(PlayerError::Unhealthy(issue)) => {
            assert_eq!(issue, attractor_core::SimulationHealthIssue::Collapsed)
        }
        other => panic!("expected a collapsed run, got {other:?}"),
    }
}

#[test]
fn a_degenerate_run_is_replaced_when_healing_is_on() {
    let mut selection = lorenz();
    selection.initial_state = State::new(0.0, 0.0, 0.0);
    let options = PlayerOptions {
        heal_automatically: true,
        ..pinned_options()
    };
    let mut player = Player::new(selection, validated_catalog(), options, SplitMix64::new(2))
        .expect("the player builds");

    let mut event = TickEvent::Advanced;
    for _ in 0..5_000 {
        event = player.tick(0.1).expect("healing keeps the run going");
        if event != TickEvent::Advanced {
            break;
        }
    }
    assert_eq!(
        event,
        TickEvent::Restarted,
        "the collapsed run must be replaced"
    );
    assert!(player.frames_simulated() > 0);
}

#[test]
fn restarts_are_limited_and_deferred() {
    let options = PlayerOptions {
        heal_automatically: true,
        ..pinned_options()
    };
    let mut player = player(11, options);
    for _ in 0..3 {
        assert_eq!(
            player.restart().expect("the restart runs"),
            RestartOutcome::Restarted
        );
    }
    match player.restart().expect("the limiter still answers") {
        RestartOutcome::Blocked {
            retry_after_seconds,
        } => assert!(
            retry_after_seconds > 0.0 && retry_after_seconds <= 10.0,
            "retry after {retry_after_seconds}"
        ),
        other => panic!("expected a blocked restart, got {other:?}"),
    }

    // Once the limiter window has elapsed, restarts are allowed again.
    for _ in 0..120 {
        player.tick(0.1).expect("the healthy run advances");
    }
    assert_eq!(
        player.restart().expect("the restart runs"),
        RestartOutcome::Restarted
    );
}

#[test]
fn resizing_updates_the_frame_size_and_rejects_zero() {
    let mut player = player(1, pinned_options());
    assert_eq!(player.size(), (320, 240));
    player.resize(640, 480, 2.0).expect("the resize applies");
    assert_eq!(player.size(), (640, 480));
    assert!(matches!(
        player.resize(0, 480, 1.0),
        Err(PlayerError::Gpu(_))
    ));
    assert_eq!(player.size(), (640, 480));
}

#[test]
fn a_rendered_frame_is_not_blank() {
    let context = match GpuContext::new() {
        Ok(context) => context,
        Err(error) => {
            eprintln!("skipping the render test: {error}");
            return;
        }
    };
    let mut player = player(
        42,
        PlayerOptions {
            overlay_scale: 2.0,
            ..pinned_options()
        },
    );
    player.warm_up(5_000).expect("warm-up stays healthy");
    for _ in 0..60 {
        player.tick(FRAME_INTERVAL).expect("frame advances");
    }
    let pixels = player.render(&context).expect("the frame renders");
    assert_eq!(pixels.len(), 320 * 240 * 4);
    assert!(lit_pixels(&pixels) > 0, "the ribbon must draw ink");

    player
        .resize(160, 120, 1.0)
        .expect("the renderer accepts a resize");
    let pixels = player.render(&context).expect("the resized frame renders");
    assert_eq!(pixels.len(), 160 * 120 * 4);
    assert!(lit_pixels(&pixels) > 0, "the resized ribbon must draw ink");
}
