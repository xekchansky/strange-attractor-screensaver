//! Golden camera poses and ribbon geometry from the Swift oracle.
//!
//! `tests/fixtures/view_fixtures.json` records the `CameraController`'s
//! `SCNNode` positions over fourteen frames and the exact `SCNGeometry`
//! buffers `TraceRenderer` builds for every color mode. The generator lives in
//! `Render/Tests/AttractorRenderTests/ViewFixtureDumpTests.swift`; see
//! `tests/fixtures/README.md` for the command that reproduces it.

use attractor_core::{State, TraceBuffer, TraceColorMode};
use attractor_view::{CameraController, TrailBuilder, Vector3};

const FIXTURE: &str = include_str!("fixtures/view_fixtures.json");

#[derive(Debug, serde::Deserialize)]
struct Fixture {
    camera: CameraFixture,
    trail: TrailFixture,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CameraFixture {
    azimuth: String,
    elevation: String,
    dt: String,
    rotation_speed: String,
    states: Vec<FixtureState>,
    frames: Vec<CameraFrame>,
}

#[derive(Debug, serde::Deserialize)]
struct CameraFrame {
    position: FixtureVector,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrailFixture {
    camera_position: FixtureVector,
    states: Vec<FixtureState>,
    cases: Vec<TrailCase>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrailCase {
    mode: String,
    color_offset: String,
    vertex_count: usize,
    positions: Vec<String>,
    colors: Vec<String>,
    indices: Vec<u32>,
    primitive_count: usize,
}

#[derive(Debug, serde::Deserialize)]
struct FixtureState {
    x: String,
    y: String,
    z: String,
}

#[derive(Debug, serde::Deserialize)]
struct FixtureVector {
    x: String,
    y: String,
    z: String,
}

fn number(text: &str) -> f64 {
    text.parse().expect("fixture digit")
}

impl FixtureState {
    fn state(&self) -> State {
        State::new(number(&self.x), number(&self.y), number(&self.z))
    }
}

impl FixtureVector {
    fn vector(&self) -> Vector3 {
        Vector3::new(number(&self.x), number(&self.y), number(&self.z))
    }
}

fn trace_from(states: &[State]) -> TraceBuffer {
    let mut trace = TraceBuffer::new(states.len(), states[0]);
    for &state in &states[1..] {
        trace.append(state);
    }
    trace
}

fn mode(name: &str) -> TraceColorMode {
    match name {
        "trail" => TraceColorMode::Trail,
        "age" => TraceColorMode::Age,
        "depth" => TraceColorMode::Depth,
        "spatial" => TraceColorMode::Spatial,
        other => panic!("unknown color mode {other}"),
    }
}

#[test]
fn the_fixture_file_is_parseable() {
    let fixture: Fixture = serde_json::from_str(FIXTURE).expect("view fixture");
    assert_eq!(fixture.camera.states.len(), 9);
    assert_eq!(fixture.camera.frames.len(), 14);
    assert_eq!(fixture.trail.states.len(), 7);
    assert_eq!(fixture.trail.cases.len(), 5);
}

#[test]
fn the_camera_reproduces_the_swift_node_positions() {
    let fixture: Fixture = serde_json::from_str(FIXTURE).expect("view fixture");
    let states: Vec<State> = fixture
        .camera
        .states
        .iter()
        .map(FixtureState::state)
        .collect();
    let trace = trace_from(&states);
    let mut camera = CameraController::new(
        number(&fixture.camera.azimuth),
        number(&fixture.camera.elevation),
    );

    for (frame_index, frame) in fixture.camera.frames.iter().enumerate() {
        camera.update(
            number(&fixture.camera.dt),
            number(&fixture.camera.rotation_speed),
            &trace,
        );
        let expected = frame.position.vector();
        let actual = camera.position();
        assert_eq!(
            actual, expected,
            "frame {frame_index}: camera {actual:?} vs Swift node {expected:?}"
        );
    }
}

#[test]
fn the_trail_reproduces_the_swift_geometry_buffers() {
    let fixture: Fixture = serde_json::from_str(FIXTURE).expect("view fixture");
    let states: Vec<State> = fixture
        .trail
        .states
        .iter()
        .map(FixtureState::state)
        .collect();
    let trace = trace_from(&states);
    let camera_position = fixture.trail.camera_position.vector();
    let mut builder = TrailBuilder::new();

    for case in &fixture.trail.cases {
        let geometry = builder.build(
            &trace,
            number(&case.color_offset),
            mode(&case.mode),
            camera_position,
        );

        assert_eq!(
            geometry.vertex_count(),
            case.vertex_count,
            "{} vertex count",
            case.mode
        );
        assert_eq!(
            geometry.positions().len() * 3,
            case.positions.len(),
            "{} position components",
            case.mode
        );
        for (index, position) in geometry.positions().iter().enumerate() {
            assert_eq!(
                position.x,
                number(&case.positions[index * 3]),
                "{} vertex {index} x",
                case.mode
            );
            assert_eq!(
                position.y,
                number(&case.positions[index * 3 + 1]),
                "{} vertex {index} y",
                case.mode
            );
            assert_eq!(
                position.z,
                number(&case.positions[index * 3 + 2]),
                "{} vertex {index} z",
                case.mode
            );
        }

        assert_eq!(geometry.colors().len() * 3, case.colors.len());
        for (index, color) in geometry.colors().iter().enumerate() {
            assert_eq!(
                f64::from(color.r),
                number(&case.colors[index * 3]),
                "{} vertex {index} red",
                case.mode
            );
            assert_eq!(
                f64::from(color.g),
                number(&case.colors[index * 3 + 1]),
                "{} vertex {index} green",
                case.mode
            );
            assert_eq!(
                f64::from(color.b),
                number(&case.colors[index * 3 + 2]),
                "{} vertex {index} blue",
                case.mode
            );
        }

        assert_eq!(
            geometry.indices(),
            case.indices.as_slice(),
            "{} indices",
            case.mode
        );
        assert_eq!(
            geometry.triangle_count(),
            case.primitive_count,
            "{} primitive count",
            case.mode
        );
    }
}
