//! The browser half: a canvas surface, one player and its page-facing API.
//!
//! JavaScript owns the frame loop (`requestAnimationFrame`), the DOM overlay
//! and the URL controls; this module only advances the simulation, presents a
//! frame and exposes the state the page displays. Keeping the loop in
//! JavaScript avoids a second timer abstraction in Rust and matches the I0.5
//! spike, where the loop lived in the page as well.

use attractor_core::SplitMix64;
use attractor_core::catalog;
use attractor_gpu::{GpuContext, SAMPLE_COUNT, SurfaceTarget};
use attractor_player::{Player, PlayerOptions};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::catalog_view;
use crate::options::{Backend, DemoOptions};
use crate::selection;

/// The catalog as JSON: `[{"id": …, "name": …}, …]`.
///
/// The page fills its attractor picker from this list.
#[wasm_bindgen]
pub fn catalog_json() -> Result<String, JsValue> {
    let catalog = catalog::bundled().map_err(js_error)?;
    Ok(catalog_view::catalog_json(&catalog))
}

/// Start the demo on `canvas`.
///
/// `query` is the page's `location.search`; see [`DemoOptions`] for the
/// accepted keys. `fallback_seed` is used when the URL carries no seed, so the
/// page can pass a fresh random one while keeping the URL as the single source
/// of state.
#[wasm_bindgen]
pub async fn start(
    canvas: HtmlCanvasElement,
    query: String,
    fallback_seed: u64,
) -> Result<Demo, JsValue> {
    console_error_panic_hook::set_once();

    let options = DemoOptions::parse(&query);
    let catalog = catalog::bundled().map_err(js_error)?;
    let run = selection::build(&catalog, &options, fallback_seed).map_err(js_error)?;

    let width = canvas.width().max(1);
    let height = canvas.height().max(1);
    let (context, target) =
        SurfaceTarget::create_web(canvas, width, height, options.backend == Backend::WebGl2)
            .await
            .map_err(js_error)?;
    let backend = context.backend_label();
    let sample_count = if context.supports_multisampling() {
        SAMPLE_COUNT
    } else {
        1
    };

    let player = Player::new(
        run.selection,
        catalog,
        PlayerOptions {
            camera_azimuth: None,
            camera_elevation: None,
            heal_automatically: true,
            width,
            height,
            // The overlay is DOM text on the web (D8), so this only has to stay
            // a sane positive number for the player's bookkeeping.
            overlay_scale: 1.0,
            sample_count,
        },
        SplitMix64::new(run.seed),
    )
    .map_err(js_error)?;

    Ok(Demo {
        context,
        target,
        player,
        backend,
    })
}

/// One running demo page.
#[wasm_bindgen]
pub struct Demo {
    context: GpuContext,
    target: SurfaceTarget,
    player: Player,
    backend: &'static str,
}

#[wasm_bindgen]
impl Demo {
    /// Advance the simulation by `dt` seconds of wall-clock time.
    pub fn tick(&mut self, dt: f64) -> Result<(), JsValue> {
        self.player.tick(dt).map(|_| ()).map_err(js_error)
    }

    /// Draw and present the current frame.
    pub fn render(&mut self) -> Result<(), JsValue> {
        self.player
            .render_to_surface(&self.context, &mut self.target)
            .map_err(js_error)
    }

    /// Retarget the player and the swap chain at a new canvas size.
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), JsValue> {
        self.player
            .resize(width.max(1), height.max(1), 1.0)
            .map_err(js_error)
    }

    /// The overlay text: name, formulas, parameters and the live position.
    pub fn overlay(&self) -> String {
        self.player.overlay().text()
    }

    /// Id of the attractor currently running.
    pub fn attractor_id(&self) -> String {
        self.player.selection().descriptor.id.clone()
    }

    /// Display name of the attractor currently running.
    pub fn attractor_name(&self) -> String {
        self.player.selection().descriptor.name.clone()
    }

    /// Backend the demo actually uses (`webgpu`, `webgl2`, …).
    pub fn backend(&self) -> String {
        self.backend.to_string()
    }

    /// Adapter name reported by the browser.
    pub fn adapter(&self) -> String {
        self.context.adapter_info().name.clone()
    }

    /// Frames presented so far.
    pub fn frames(&self) -> u32 {
        u32::try_from(self.player.frames_simulated()).unwrap_or(u32::MAX)
    }
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}
