//! The record loop must settle the camera before the first captured frame,
//! exactly like the Swift `CaptureController`.

use attractor_cli::{DEFAULT_SETTLE_FRAMES, RunError, SimulationSession, record_sequence};
use attractor_core::{AttractorSelection, DebugConfig, SplitMix64, catalog};

fn session() -> SimulationSession {
    let catalog = catalog::bundled().expect("catalog decodes");
    let selection: AttractorSelection =
        DebugConfig::selection("lorenz", &catalog).expect("lorenz builds");
    let mut rng = SplitMix64::new(42);
    SimulationSession::new(selection, None, None, &mut rng).expect("session builds")
}

#[test]
fn record_settles_the_camera_before_the_first_frame() {
    let mut session = session();
    let mut captured = Vec::new();
    record_sequence(&mut session, 3, 2, |session, index| {
        captured.push((index, session.frames_simulated()));
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
    let mut session = session();
    let mut indices = Vec::new();
    record_sequence(&mut session, 4, 3, |_, index| {
        indices.push(index);
        Ok::<(), RunError>(())
    })
    .expect("the record sequence runs");

    assert_eq!(indices, vec![1, 2, 3, 4]);
    assert_eq!(
        session.frames_simulated(),
        DEFAULT_SETTLE_FRAMES + 4 * 3,
        "the sequence must stop after the requested number of frames"
    );
}
