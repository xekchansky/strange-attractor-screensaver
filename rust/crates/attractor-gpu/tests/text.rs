//! Text overlay layout. Needs a GPU adapter and system fonts; skips without
//! either.

#![cfg(feature = "text")]

use attractor_gpu::{FrameRenderer, GpuContext, TextOverlay, lit_pixels};
use attractor_view::{Mat4, TrailGeometry};

fn context() -> Option<GpuContext> {
    match GpuContext::new() {
        Ok(context) => Some(context),
        Err(error) => {
            eprintln!("skipping the text overlay test: {error}");
            None
        }
    }
}

/// Lit pixels in the lower half of a tightly packed RGBA8 frame. The overlay
/// sits in the lower-left corner, and readback rows start at the top.
fn lit_in_lower_half(pixels: &[u8], width: u32, height: u32) -> usize {
    let row_bytes = (width * 4) as usize;
    (height as usize / 2..height as usize)
        .map(|row| {
            let start = row * row_bytes;
            lit_pixels(&pixels[start..start + row_bytes])
        })
        .sum()
}

#[test]
fn overlay_text_lays_out_on_a_gpu_context() {
    let Some(context) = context() else { return };
    let mut overlay = TextOverlay::new(&context, attractor_gpu::TARGET_FORMAT, 2.0, 1);
    overlay
        .prepare(
            &context,
            "Lorenz\ndX/dt = 10 * (y - x)\ndY/dt = x * (28 - z) - y\n\
             dZ/dt = x * y - 2.667 * z\n\
             b = 2.667   sigma = 10.000\n\
             (x, y, z) = (1.000, 2.000, 3.000)",
            1200,
            900,
        )
        .expect("the overlay lays out");
    overlay.trim();
}

#[test]
fn the_overlay_draws_glyphs_into_the_lower_left_corner() {
    let Some(context) = context() else { return };
    let mut renderer = FrameRenderer::new(&context, 320, 240, 1.0).expect("frame renderer");
    let projection = Mat4::identity();
    let blank = renderer
        .render(&context, &TrailGeometry::default(), &projection, None)
        .expect("a frame without text");
    assert_eq!(
        lit_pixels(&blank),
        0,
        "an empty ribbon must leave only the clear color"
    );

    let pixels = renderer
        .render(
            &context,
            &TrailGeometry::default(),
            &projection,
            Some("Lorenz\n(x, y, z) = (1.000, 2.000, 3.000)"),
        )
        .expect("a frame with text");
    let ink = lit_in_lower_half(&pixels, 320, 240);
    assert!(
        ink > 0,
        "the overlay drew no glyphs; is a system font available?"
    );
}

#[test]
fn empty_overlay_text_lays_out() {
    let Some(context) = context() else { return };
    let mut overlay = TextOverlay::new(&context, attractor_gpu::TARGET_FORMAT, 1.0, 1);
    overlay
        .prepare(&context, "", 64, 48)
        .expect("empty text lays out");
}
