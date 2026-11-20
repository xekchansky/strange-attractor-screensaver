//! Self-overlap: the nearer ribbon must win wherever the trail crosses itself.

use attractor_core::{State, TraceBuffer, TraceColorMode};
use attractor_gpu::{FrameRenderer, GpuContext, SAMPLE_COUNT};
use attractor_view::{Mat4, TrailBuilder, Vector3};

/// Frame edge in pixels; large enough that the far pass is many pixels wide.
const SIZE: u32 = 512;
/// Half-width of the crossing passes.
///
/// The ribbon radius is a fixed fraction (0.4%) of the trail's bounding
/// extent, so the passes have to span a wide range to come out several pixels
/// thick at the camera distance used below.
const HALF_SPAN: f64 = 20.0;
/// Depth of the far pass. The near pass sits at z = 0.
const FAR_Z: f64 = -1.0;

/// Build a trail that runs across the middle at `z = 0` and then runs the
/// other way across the middle at `FAR_Z`, so the far pass is rasterized last.
fn crossing_trail() -> TraceBuffer {
    let mut trace = TraceBuffer::new(16, State::new(-HALF_SPAN, 0.0, 0.0));
    for (x, z) in [
        (0.0, 0.0),
        (HALF_SPAN, 0.0),
        (HALF_SPAN, FAR_Z),
        (0.0, FAR_Z),
        (-HALF_SPAN, FAR_Z),
    ] {
        trace.append(State::new(x, 0.0, z));
    }
    trace
}

#[test]
fn the_nearer_ribbon_wins_where_the_trail_crosses_itself() {
    let context = match GpuContext::new() {
        Ok(context) => context,
        Err(error) => {
            eprintln!("skipping the occlusion test: {error}");
            return;
        }
    };

    let camera = Vector3::new(0.0, 0.0, 0.5);
    let mut builder = TrailBuilder::new();
    // Spatial coloring encodes the coordinate in the channel: z maps to blue,
    // so the near pass (z = 0) is blue and the far pass (z = FAR_Z) has none.
    let geometry = builder
        .build(&crossing_trail(), 0.0, TraceColorMode::Spatial, camera)
        .clone();

    let view = Mat4::look_at(
        camera,
        Vector3::new(0.0, 0.0, FAR_Z - 0.5),
        Vector3::new(0.0, 1.0, 0.0),
    );
    let view_projection =
        Mat4::perspective(std::f64::consts::PI / 3.0, 1.0, 0.01, 100.0).multiply(&view);

    let mut renderer = FrameRenderer::with_sample_count(&context, SIZE, SIZE, 1.0, SAMPLE_COUNT)
        .expect("the offscreen renderer builds");
    let pixels = renderer
        .render(&context, &geometry, &view_projection, None)
        .expect("the frame renders");

    let center = ((SIZE / 2) * SIZE + SIZE / 2) as usize * 4;
    let (red, green, blue) = (pixels[center], pixels[center + 1], pixels[center + 2]);
    assert!(
        blue > 128,
        "the near pass must stay visible where the far pass is behind it, \
         but the centre pixel is ({red}, {green}, {blue})"
    );
}
