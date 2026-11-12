//! The record loop must settle the camera before the first captured frame,
//! exactly like the Swift `CaptureController`.

use attractor_cli::{DEFAULT_SETTLE_FRAMES, RunError, record_sequence};
use attractor_core::{DebugConfig, SplitMix64, catalog};
use attractor_player::{Player, PlayerOptions};

fn player() -> Player {
    let validation = catalog::bundled_validation().expect("catalog validates");
    let selection = DebugConfig::selection("lorenz", &validation.valid).expect("lorenz builds");
    Player::new(
        selection,
        validation.valid,
        PlayerOptions {
            camera_azimuth: Some(0.0),
            camera_elevation: Some(0.0),
            heal_automatically: false,
            ..PlayerOptions::default()
        },
        SplitMix64::new(42),
    )
    .expect("the player builds")
}

#[test]
fn record_settles_the_camera_before_the_first_frame() {
    let mut player = player();
    let mut captured = Vec::new();
    record_sequence(&mut player, 3, 2, |player, index| {
        captured.push((index, player.frames_simulated()));
        Ok::<(), RunError>(())
    })
    .expect("the record sequence runs");

    assert_eq!(
        captured,
        vec![
            (1, DEFAULT_SETTLE_FRAMES + 2),
            (2, DEFAULT_SETTLE_FRAMES + 4),
            (3, DEFAULT_SETTLE_FRAMES + 6),
        ]
    );
}

#[test]
fn record_writes_every_stride_frame_in_order() {
    let mut player = player();
    let mut indices = Vec::new();
    record_sequence(&mut player, 4, 3, |_, index| {
        indices.push(index);
        Ok::<(), RunError>(())
    })
    .expect("the record sequence runs");

    assert_eq!(indices, vec![1, 2, 3, 4]);
    assert_eq!(
        player.frames_simulated(),
        DEFAULT_SETTLE_FRAMES + 4 * 3,
        "the sequence must stop after the requested number of frames"
    );
}
