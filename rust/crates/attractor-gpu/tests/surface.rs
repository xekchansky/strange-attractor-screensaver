//! Surface rendering: format selection and the external-target pipeline.

use attractor_gpu::{
    FrameRenderer, GpuContext, GpuError, TARGET_FORMAT, choose_surface_format, lit_pixels,
};
use attractor_view::{Mat4, TrailGeometry};
use wgpu::TextureFormat;

#[test]
fn the_surface_format_prefers_srgb() {
    assert_eq!(
        choose_surface_format(&[TextureFormat::Bgra8UnormSrgb, TextureFormat::Rgba8UnormSrgb]),
        Some(TextureFormat::Bgra8UnormSrgb)
    );
    assert_eq!(
        choose_surface_format(&[TextureFormat::Rgba8Unorm, TextureFormat::Rgba8UnormSrgb]),
        Some(TextureFormat::Rgba8UnormSrgb)
    );
    assert_eq!(
        choose_surface_format(&[TextureFormat::Rgba8Unorm]),
        Some(TextureFormat::Rgba8Unorm)
    );
    assert_eq!(choose_surface_format(&[]), None);
}

#[test]
fn a_surface_renderer_draws_into_an_external_texture() {
    let context = GpuContext::new_noop().expect("the noop backend is available");
    let format = TextureFormat::Bgra8UnormSrgb;
    let mut renderer = FrameRenderer::for_surface(&context, format, 64, 48, 2.0, 1)
        .expect("the surface pipeline builds");
    assert_eq!(renderer.format(), format);
    assert_eq!(renderer.size(), (64, 48));

    let texture = context.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("test surface drawable"),
        size: wgpu::Extent3d {
            width: 64,
            height: 48,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    renderer
        .render_to_view(
            &context,
            &view,
            &TrailGeometry::default(),
            &Mat4::identity(),
            Some("Lorenz"),
        )
        .expect("the frame is recorded");
}

#[test]
fn a_surface_renderer_has_nothing_to_read_back() {
    let context = GpuContext::new_noop().expect("the noop backend is available");
    let mut renderer = FrameRenderer::for_surface(&context, TARGET_FORMAT, 16, 16, 1.0, 1)
        .expect("the surface pipeline builds");
    let error = renderer
        .render(&context, &TrailGeometry::default(), &Mat4::identity(), None)
        .expect_err("a surface renderer cannot read back");
    assert!(matches!(error, GpuError::NoReadbackTarget));
}

/// Keeps the public surface of the crate honest: the offscreen renderer still
/// produces ink after the external-target split.
#[test]
fn the_offscreen_renderer_still_reads_back_ink() {
    let context = match GpuContext::new() {
        Ok(context) => context,
        Err(error) => {
            eprintln!("skipping the offscreen render test: {error}");
            return;
        }
    };
    let mut renderer = FrameRenderer::with_sample_count(&context, 64, 48, 1.0, 1)
        .expect("the offscreen renderer builds");
    let pixels = renderer
        .render(
            &context,
            &TrailGeometry::default(),
            &Mat4::identity(),
            Some("Lorenz"),
        )
        .expect("the frame is read back");
    assert_eq!(pixels.len(), 64 * 48 * 4);
    assert!(lit_pixels(&pixels) > 0, "the overlay must ink the frame");
}
