//! wgpu device, ribbon pipeline, text overlay and offscreen capture.
//!
//! The crate owns every GPU-facing concern of the native renderer: creating a
//! headless device, uploading the ribbon buffers built by `attractor-view`,
//! drawing them into an sRGB target with the engine's text overlay, and
//! reading the frame back as RGBA8 pixels. It never touches the simulation or
//! the platform window, so the same pipeline will back the macOS `.saver`
//! (I7a/I7b), the Windows and Linux shells (I8/I10) and the wasm demo (I9).

mod capture;
mod context;
mod frame;
mod ribbon;
mod surface;
#[cfg(feature = "text")]
mod text;
mod vertex;

pub use capture::{lit_pixels, write_png};
pub use context::{GpuContext, GpuError};
pub use frame::FrameRenderer;
pub use ribbon::{RIBBON_SHADER, RibbonRenderer};
pub use surface::{SurfaceTarget, choose_surface_format};
#[cfg(feature = "text")]
pub use text::TextOverlay;
pub use vertex::Vertex;

/// The sRGB render target every capture uses.
pub const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
/// Multisample count of the ribbon and text pipelines.
///
/// The Swift host snapshots with 4x multisampling, so edges get the same
/// antialiasing treatment. The WebGL2 fallback cannot resolve multisampled
/// targets, so the web target will build the same pipelines with a count of
/// one (I9).
pub const SAMPLE_COUNT: u32 = 4;
/// The background color of every frame (opaque black, like the Swift scene).
pub(crate) const CLEAR_COLOR: wgpu::Color = wgpu::Color::BLACK;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_is_configured() {
        assert_eq!(env!("CARGO_PKG_NAME"), "attractor-gpu");
    }
}
