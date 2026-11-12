//! Orbit camera: constants, framing cadence, easing and the view transform.

use std::f64::consts::PI;

use attractor_core::{RandomSource, SplitMix64, State, TraceBuffer};
use attractor_view::{CameraController, Vector3};

const TOLERANCE: f64 = 1e-12;

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= TOLERANCE,
        "{actual} is not within {TOLERANCE} of {expected}"
    );
}

fn trace_from(states: &[State]) -> TraceBuffer {
    let mut trace = TraceBuffer::new(states.len(), states[0]);
    for &state in &states[1..] {
        trace.append(state);
    }
    trace
}

#[test]
fn orbit_constants_match_the_swift_camera() {
    assert_eq!(CameraController::FRAMING_FACTOR, 2.1);
    assert_eq!(CameraController::MINIMUM_DISTANCE, 2.0);
    assert_eq!(CameraController::SMOOTHING, 0.06);
    assert_eq!(CameraController::FRAMING_STRIDE, 6);
    assert_eq!(CameraController::DEFAULT_ELEVATION, PI / 5.0);
}

#[test]
fn a_single_point_frames_at_the_minimum_distance() {
    let trace = trace_from(&[State::new(5.0, 0.0, 0.0)]);
    let mut camera = CameraController::new(0.0, 0.0);
    camera.update(0.0, 0.0, &trace);

    // Target distance is max(0.1 * 2.1, 2) = 2; the first update eases 6% of
    // the way from the initial distance of 8.
    assert_close(camera.distance(), 8.0 + (2.0 - 8.0) * 0.06);
    // The target center is the point itself; the smoothed center eases 6%.
    assert_close(camera.center().x, 5.0 * 0.06);
    // At azimuth and elevation zero the camera sits on +x from the center.
    // The stored position is rounded through f32, like a SceneKit node.
    assert_eq!(camera.position().x, f64::from(7.94f32));
    assert_close(camera.position().y, 0.0);
    assert_close(camera.position().z, 0.0);
}

#[test]
fn update_advances_the_azimuth_by_the_rotation_speed() {
    let trace = trace_from(&[State::new(0.0, 0.0, 0.0)]);
    let mut camera = CameraController::new(0.25, PI / 4.0);
    for _ in 0..5 {
        camera.update(0.02, 0.35, &trace);
    }
    assert_close(camera.azimuth(), 0.25 + 0.35 * 0.02 * 5.0);
    assert_close(camera.elevation(), PI / 4.0);
    assert_eq!(camera.elevation(), PI / 4.0);
}

#[test]
fn framing_refreshes_only_every_sixth_frame() {
    let first = trace_from(&[State::new(0.0, 0.0, 0.0), State::new(1.0, 0.0, 0.0)]);
    let second = trace_from(&[State::new(0.0, 0.0, 0.0), State::new(100.0, 0.0, 0.0)]);
    let mut camera = CameraController::new(0.0, 0.0);

    // Frame 1 computes the target for the small trace and starts easing.
    camera.update(0.0, 0.0, &first);
    // Frames 2-6 keep easing toward the same target even though the trace has
    // grown: the framing scan is cached for six frames.
    for _ in 0..5 {
        camera.update(0.0, 0.0, &second);
    }
    let distance_before_refresh = camera.distance();
    assert!(
        distance_before_refresh < 7.0,
        "still easing toward 2, got {distance_before_refresh}"
    );

    // Frame 7 refreshes the target toward the large trace and the distance
    // jumps instead of continuing to shrink.
    camera.update(0.0, 0.0, &second);
    let distance_after_refresh = camera.distance();
    assert!(
        distance_after_refresh > distance_before_refresh + 5.0,
        "target refreshed: before {distance_before_refresh}, after {distance_after_refresh}"
    );
    assert_close(
        distance_after_refresh,
        distance_before_refresh + (105.0 - distance_before_refresh) * 0.06,
    );
}

#[test]
fn the_smoothed_center_eases_toward_the_trace_center() {
    let trace = trace_from(&[State::new(0.0, 0.0, 0.0), State::new(1.0, 0.0, 0.0)]);
    let mut camera = CameraController::new(0.0, 0.0);
    camera.update(0.0, 0.0, &trace);
    assert_close(camera.center().x, 0.5 * 0.06);
    camera.update(0.0, 0.0, &trace);
    assert_close(camera.center().x, 0.5 * 0.06 + (0.5 - 0.5 * 0.06) * 0.06);
}

#[test]
fn random_azimuth_uses_the_swift_half_open_range_draw() {
    // The screensaver starts at `Double.random(in: 0..<2 * .pi)`: Swift draws
    // 53 bits below 2^53 and maps them into the half-open range.
    let mut source = SplitMix64::new(7);
    let camera = CameraController::random(&mut source);
    assert_eq!(camera.elevation(), CameraController::DEFAULT_ELEVATION);
    assert!((0.0..2.0 * PI).contains(&camera.azimuth()));

    let mut reference = SplitMix64::new(7);
    let unit = reference.next_below(1u64 << 53) as f64 * (f64::EPSILON / 2.0);
    assert_eq!(camera.azimuth(), unit * (2.0 * PI));
}

#[test]
fn random_azimuth_is_deterministic_for_a_seed() {
    let first = CameraController::random(&mut SplitMix64::new(42));
    let second = CameraController::random(&mut SplitMix64::new(42));
    assert_eq!(first.azimuth(), second.azimuth());

    let different = CameraController::random(&mut SplitMix64::new(43));
    assert_ne!(different.azimuth(), first.azimuth());
}

#[test]
fn the_camera_position_is_quantized_to_the_scenekit_storage_grid() {
    let trace = trace_from(&[State::new(5.0, -3.0, 2.0)]);
    let mut camera = CameraController::new(0.7, PI / 5.0);
    camera.update(1.0 / 60.0, 0.35, &trace);
    let position = camera.position();
    assert_eq!(position.x, f64::from(position.x as f32));
    assert_eq!(position.y, f64::from(position.y as f32));
    assert_eq!(position.z, f64::from(position.z as f32));
}

#[test]
fn the_view_matrix_looks_from_the_camera_position_at_the_center() {
    let trace = trace_from(&[State::new(1.0, 2.0, 3.0), State::new(4.0, -1.0, 0.5)]);
    let mut camera = CameraController::new(0.4, PI / 5.0);
    for _ in 0..3 {
        camera.update(1.0 / 60.0, 0.35, &trace);
    }
    let matrix = camera.view_matrix();
    let eye = matrix.transform_point(camera.position());
    assert!(
        eye.length() < 1e-5,
        "camera sits at the origin, got {eye:?}"
    );
    let target = matrix.transform_point(camera.center());
    assert!(target.x.abs() < 1e-5);
    assert!(target.y.abs() < 1e-5);
    assert!((target.z + camera.distance()).abs() < 1e-4);
}

#[test]
fn the_camera_position_before_the_first_update_is_the_origin() {
    let camera = CameraController::new(0.5, PI / 5.0);
    assert_eq!(camera.position(), Vector3::ZERO);
}
