//! Text overlay layout. Needs a GPU adapter and system fonts; skips without
//! either.

#![cfg(feature = "text")]

use attractor_gpu::{GpuContext, TextOverlay};

fn context() -> Option<GpuContext> {
    match GpuContext::new() {
        Ok(context) => Some(context),
        Err(error) => {
            eprintln!("skipping the text overlay test: {error}");
            None
        }
    }
}

#[test]
fn overlay_text_lays_out_on_a_gpu_context() {
    let Some(context) = context() else { return };
    let mut overlay = TextOverlay::new(&context, 2.0, 1);
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
fn empty_overlay_text_lays_out() {
    let Some(context) = context() else { return };
    let mut overlay = TextOverlay::new(&context, 1.0, 1);
    overlay
        .prepare(&context, "", 64, 48)
        .expect("empty text lays out");
}
