//! Trail geometry: sampling budget, ribbon topology and color-mode rules.

use attractor_core::{State, TraceBuffer, TraceColorMode};
use attractor_view::{TrailBuilder, Vector3};

const CAMERA: Vector3 = Vector3::new(4.0, 3.0, 5.0);

fn trace_from(states: &[State]) -> TraceBuffer {
    let mut trace = TraceBuffer::new(states.len(), states[0]);
    for &state in &states[1..] {
        trace.append(state);
    }
    trace
}

fn long_state(index: usize) -> State {
    let value = index as f64;
    State::new(value * 0.001, (value * 0.01).sin(), (value * 0.013).cos())
}

fn long_trace(count: usize) -> TraceBuffer {
    let mut trace = TraceBuffer::new(count, long_state(0));
    for index in 1..count {
        trace.append(long_state(index));
    }
    trace
}

fn vertex_center(geometry: &attractor_view::TrailGeometry, point: usize) -> Vector3 {
    let positions = geometry.positions();
    positions[point * 2].lerp(positions[point * 2 + 1], 0.5)
}

#[test]
fn a_single_point_produces_no_geometry() {
    let trace = trace_from(&[State::new(1.0, 2.0, 3.0)]);
    let mut builder = TrailBuilder::new();
    let geometry = builder.build(&trace, 0.0, TraceColorMode::Trail, CAMERA);
    assert!(geometry.is_empty());
    assert_eq!(geometry.vertex_count(), 0);
    assert_eq!(geometry.point_count(), 0);
    assert_eq!(geometry.triangle_count(), 0);
    assert_eq!(geometry.ribbon_radius(), 0.0);
    assert!(geometry.positions().is_empty());
    assert!(geometry.colors().is_empty());
    assert!(geometry.indices().is_empty());
}

#[test]
fn two_points_produce_a_two_triangle_strip() {
    let trace = trace_from(&[State::new(0.0, 0.0, 0.0), State::new(1.0, 0.0, 0.0)]);
    let mut builder = TrailBuilder::new();
    let geometry = builder.build(&trace, 0.0, TraceColorMode::Trail, CAMERA);
    assert_eq!(geometry.point_count(), 2);
    assert_eq!(geometry.vertex_count(), 4);
    assert_eq!(geometry.triangle_count(), 2);
    assert_eq!(geometry.indices(), &[0, 1, 2, 3]);
    assert_eq!(geometry.colors().len(), 4);
}

#[test]
fn a_short_trail_is_drawn_untouched() {
    let states: Vec<State> = (0..20).map(long_state).collect();
    let trace = trace_from(&states);
    let mut builder = TrailBuilder::with_budget(20);
    let geometry = builder.build(&trace, 0.0, TraceColorMode::Trail, CAMERA);
    assert_eq!(geometry.point_count(), 20);
    assert_eq!(geometry.vertex_count(), 40);
}

#[test]
fn a_long_trail_is_uniformly_subsampled_and_keeps_the_head() {
    let trace = long_trace(20_001);
    let mut builder = TrailBuilder::with_budget(100);
    let geometry = builder.build(&trace, 0.0, TraceColorMode::Trail, CAMERA);

    // Stride is ceil(20001 / 100) = 201: indices 0, 201, ..., 19899 (100
    // points) plus the head at 20000.
    assert_eq!(geometry.point_count(), 101);
    assert_eq!(geometry.vertex_count(), 202);
    assert_eq!(geometry.triangle_count(), 200);
    let expected_indices: Vec<u32> = (0..202).collect();
    assert_eq!(geometry.indices(), expected_indices.as_slice());

    // The first and last sampled points are the oldest and newest states.
    let first = vertex_center(geometry, 0);
    assert!((first.x - long_state(0).x).abs() < 1e-5);
    assert!((first.y - long_state(0).y).abs() < 1e-5);
    let last = vertex_center(geometry, geometry.point_count() - 1);
    let expected_head = long_state(20_000);
    assert!((last.x - expected_head.x).abs() < 1e-5);
    assert!((last.y - expected_head.y).abs() < 1e-5);
    assert!((last.z - expected_head.z).abs() < 1e-5);
}

#[test]
fn the_head_is_preserved_even_when_the_stride_skips_it() {
    let trace = long_trace(11);
    let mut builder = TrailBuilder::with_budget(3);
    let geometry = builder.build(&trace, 0.0, TraceColorMode::Trail, CAMERA);
    // Stride 4 samples 0, 4, 8, then appends the head 10.
    assert_eq!(geometry.point_count(), 4);
    let head = vertex_center(geometry, 3);
    let expected = long_state(10);
    assert!((head.x - expected.x).abs() < 1e-5);
    assert!((head.y - expected.y).abs() < 1e-5);
    assert!((head.z - expected.z).abs() < 1e-5);
}

#[test]
fn the_ribbon_radius_is_a_share_of_the_trail_extent() {
    let trace = trace_from(&[
        State::new(-1000.0, 0.0, 0.0),
        State::new(0.0, 1000.0, 0.0),
        State::new(0.0, 0.0, -1000.0),
    ]);
    let mut builder = TrailBuilder::new();
    let geometry = builder.build(&trace, 0.0, TraceColorMode::Trail, CAMERA);
    assert_eq!(geometry.ribbon_radius(), 1000.0 * 0.004);

    // Every ribbon vertex is offset from its sample by exactly the radius.
    for point in 0..geometry.point_count() {
        let left = geometry.positions()[point * 2];
        let right = geometry.positions()[point * 2 + 1];
        let separation = left.distance(right);
        // Vertices are stored as f32; at this scale one ulp is ~6e-5.
        assert!(
            (separation - 2.0 * geometry.ribbon_radius()).abs() < 1e-3,
            "point {point}: separation {separation}"
        );
    }
}

#[test]
fn a_degenerate_trail_keeps_a_finite_ribbon() {
    let trace = trace_from(&[State::new(0.0, 0.0, 0.0), State::new(0.0, 0.0, 0.0)]);
    let mut builder = TrailBuilder::new();
    let geometry = builder.build(&trace, 0.0, TraceColorMode::Depth, CAMERA);
    assert_eq!(geometry.ribbon_radius(), 1e-6);
    for position in geometry.positions() {
        assert!(position.x.is_finite());
        assert!(position.y.is_finite());
        assert!(position.z.is_finite());
    }
}

#[test]
fn every_vertex_carries_the_color_of_its_sample() {
    let states: Vec<State> = (0..12).map(long_state).collect();
    let trace = trace_from(&states);
    let mut builder = TrailBuilder::new();
    for mode in TraceColorMode::ALL {
        let geometry = builder.build(&trace, 0.3, mode, CAMERA);
        let colors = geometry.colors();
        assert_eq!(colors.len(), geometry.vertex_count());
        for point in 0..geometry.point_count() {
            assert_eq!(colors[point * 2], colors[point * 2 + 1]);
        }
    }
}

#[test]
fn the_spatial_mode_ignores_the_animated_offset() {
    let states: Vec<State> = (0..12).map(long_state).collect();
    let trace = trace_from(&states);
    let mut builder = TrailBuilder::new();
    let still = builder
        .build(&trace, 0.0, TraceColorMode::Spatial, CAMERA)
        .colors()
        .to_vec();
    let shifted = builder
        .build(&trace, 0.7, TraceColorMode::Spatial, CAMERA)
        .colors()
        .to_vec();
    assert_eq!(still, shifted);
}

#[test]
fn animated_modes_respond_to_the_offset() {
    let states: Vec<State> = (0..12).map(long_state).collect();
    let trace = trace_from(&states);
    let mut builder = TrailBuilder::new();
    for mode in [
        TraceColorMode::Trail,
        TraceColorMode::Age,
        TraceColorMode::Depth,
    ] {
        let before = builder.build(&trace, 0.0, mode, CAMERA).colors().to_vec();
        let after = builder.build(&trace, 0.4, mode, CAMERA).colors().to_vec();
        assert_ne!(before, after, "{mode:?} should animate");
    }
}

#[test]
fn rebuilding_replaces_the_previous_geometry() {
    let short = trace_from(&[State::new(0.0, 0.0, 0.0), State::new(1.0, 0.0, 0.0)]);
    let long = trace_from(&[
        State::new(0.0, 0.0, 0.0),
        State::new(1.0, 0.0, 0.0),
        State::new(2.0, 1.0, 0.0),
        State::new(2.0, 2.0, 1.0),
    ]);
    let mut builder = TrailBuilder::new();
    let first = builder.build(&short, 0.0, TraceColorMode::Trail, CAMERA);
    assert_eq!(first.vertex_count(), 4);
    let second = builder.build(&long, 0.0, TraceColorMode::Trail, CAMERA);
    assert_eq!(second.vertex_count(), 8);
    assert_eq!(second.indices(), &[0, 1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn the_builder_reports_its_budget() {
    assert_eq!(TrailBuilder::new().budget(), TrailBuilder::DEFAULT_BUDGET);
    assert_eq!(
        TrailBuilder::default().budget(),
        TrailBuilder::DEFAULT_BUDGET
    );
    assert_eq!(TrailBuilder::with_budget(7).budget(), 7);
    // A zero budget is raised to one so the stride stays defined.
    assert_eq!(TrailBuilder::with_budget(0).budget(), 1);
}

#[test]
fn the_default_budget_limits_the_sampled_points() {
    assert_eq!(TrailBuilder::DEFAULT_BUDGET, 16_384);
    let trace = long_trace(TrailBuilder::DEFAULT_BUDGET + 1);
    let mut builder = TrailBuilder::new();
    let geometry = builder.build(&trace, 0.0, TraceColorMode::Trail, CAMERA);
    // Stride 2 keeps ceil(16385 / 2) = 8193 samples; the head is already last.
    assert_eq!(geometry.point_count(), 8_193);
}

#[test]
fn the_builder_exposes_its_last_geometry() {
    let trace = trace_from(&[
        State::new(0.0, 0.0, 0.0),
        State::new(1.0, 0.5, 0.0),
        State::new(2.0, 0.0, -0.5),
    ]);
    let mut builder = TrailBuilder::new();
    let built = builder
        .build(&trace, 0.25, TraceColorMode::Age, CAMERA)
        .clone();
    assert_eq!(builder.geometry(), &built);
    assert_eq!(builder.geometry().point_count(), 3);
}
