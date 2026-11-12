//! Pipeline construction, vertex marshalling, PNG output and the headless
//! render smoke test.

use attractor_core::{State, TraceBuffer, TraceColorMode};
use attractor_gpu::{
    FrameRenderer, GpuContext, RibbonRenderer, SAMPLE_COUNT, TARGET_FORMAT, Vertex, lit_pixels,
    write_png,
};
use attractor_view::{Mat4, TrailBuilder, Vector3};

/// A short deterministic trace: three points a unit apart.
fn trace() -> TraceBuffer {
    let mut trace = TraceBuffer::new(8, State::new(0.0, 0.0, 0.0));
    trace.append(State::new(1.0, 0.5, -0.5));
    trace.append(State::new(2.0, -0.5, 0.5));
    trace
}

fn geometry() -> attractor_view::TrailGeometry {
    let mut builder = TrailBuilder::new();
    builder
        .build(
            &trace(),
            0.0,
            TraceColorMode::Trail,
            Vector3::new(0.0, 0.0, 5.0),
        )
        .clone()
}

#[test]
fn vertex_layout_is_position_then_color() {
    let layout = Vertex::layout();
    assert_eq!(layout.array_stride, 24);
    assert_eq!(layout.attributes.len(), 2);
    assert_eq!(layout.attributes[0].offset, 0);
    assert_eq!(layout.attributes[0].format, wgpu::VertexFormat::Float32x3);
    assert_eq!(layout.attributes[1].offset, 12);
    assert_eq!(layout.attributes[1].format, wgpu::VertexFormat::Float32x3);
}

#[test]
fn geometry_becomes_two_vertices_per_point() {
    let geometry = geometry();
    let vertices = Vertex::from_geometry(&geometry);
    assert_eq!(geometry.point_count(), 3);
    assert_eq!(vertices.len(), 6);
    // The ribbon head keeps the last trace point's average position.
    let head = geometry.positions()[4];
    let next = geometry.positions()[5];
    assert!(((head.x + next.x) / 2.0 - 2.0).abs() < 1e-6);
    assert_eq!(vertices[0].color, vertices[1].color);
}

#[test]
fn shader_is_valid_wgsl() {
    let module = naga::front::wgsl::parse_str(attractor_gpu::RIBBON_SHADER)
        .expect("the ribbon shader parses");
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    validator
        .validate(&module)
        .expect("the ribbon shader validates");
}

#[test]
fn noop_backend_builds_the_ribbon_pipeline() {
    let context = GpuContext::new_noop().expect("noop adapter");
    let mut renderer = RibbonRenderer::new(&context, TARGET_FORMAT, SAMPLE_COUNT);
    let geometry = geometry();
    renderer.upload(&context, &geometry);
    assert_eq!(renderer.vertex_count(), geometry.vertex_count() as u32);
    renderer.set_view_projection(
        &context,
        &Mat4::perspective(std::f64::consts::FRAC_PI_3, 4.0 / 3.0, 0.1, 100.0),
    );
}

#[test]
fn png_round_trip_preserves_the_pixels() {
    let width = 3u32;
    let height = 2u32;
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for index in 0..(width * height) {
        pixels.extend_from_slice(&[index as u8, 2, 3, 255]);
    }
    let path = std::env::temp_dir().join(format!(
        "attractor-gpu-png-{}-{:?}.png",
        std::process::id(),
        std::thread::current().id()
    ));
    write_png(&path, width, height, &pixels).expect("png written");
    let decoder = png::Decoder::new(std::io::BufReader::new(
        std::fs::File::open(&path).expect("png reopened"),
    ));
    let mut reader = decoder.read_info().expect("png header");
    let mut decoded = vec![0; reader.output_buffer_size().expect("buffer size")];
    let info = reader.next_frame(&mut decoded).expect("png frame");
    assert_eq!((info.width, info.height), (width, height));
    assert_eq!(info.color_type, png::ColorType::Rgba);
    assert_eq!(&decoded[..pixels.len()], &pixels[..]);
    std::fs::remove_file(path).expect("png removed");
}

#[test]
fn lit_pixels_ignores_the_black_background() {
    let background = [0u8, 0, 0, 255];
    let pixels: Vec<u8> = std::iter::repeat_n(background, 4).flatten().collect();
    assert_eq!(lit_pixels(&pixels), 0);
}

#[test]
fn lit_pixels_counts_a_colored_pixel() {
    let mut pixels: Vec<u8> = std::iter::repeat_n([0u8, 0, 0, 255], 4).flatten().collect();
    pixels[0] = 200;
    assert_eq!(lit_pixels(&pixels), 1);
}

/// The real renderer needs a working GPU adapter. CI runners without one skip
/// the test; `cargo test -- --nocapture` prints the reason.
#[test]
fn headless_frame_is_not_blank() {
    let context = match GpuContext::new() {
        Ok(context) => context,
        Err(error) => {
            eprintln!("skipping the headless render: {error}");
            return;
        }
    };
    let geometry = geometry();
    let mut renderer = FrameRenderer::new(&context, 64, 48, 1.0).expect("frame renderer");
    let view = Mat4::look_at(
        Vector3::new(0.0, 0.0, 5.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::UP,
    );
    let projection = Mat4::perspective(std::f64::consts::FRAC_PI_3, 64.0 / 48.0, 0.1, 100.0);
    let pixels = renderer
        .render(&context, &geometry, &projection.multiply(&view), None)
        .expect("frame rendered");
    assert_eq!(pixels.len(), 64 * 48 * 4);
    assert!(lit_pixels(&pixels) > 0, "the ribbon did not draw anything");
}
