//! The corner overlay drawn with glyphon.

use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, PrepareError, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport, Wrap,
};

use crate::TARGET_FORMAT;
use crate::context::GpuContext;

/// Font size of the overlay in logical pixels, matching the Swift label.
const FONT_SIZE: f32 = 13.0;
/// Line height of the overlay in logical pixels.
const LINE_HEIGHT: f32 = 16.0;
/// Distance from the view's left and bottom edges in logical pixels.
const MARGIN: f32 = 16.0;
/// The overlay text color: white on the black background.
const TEXT_COLOR: Color = Color::rgb(255, 255, 255);

/// Draws the multi-line overlay in the lower-left corner.
///
/// The overlay content comes from `attractor_view::OverlayModel`; this type
/// only lays it out and renders it. A capture at scale 2 scales the font and
/// margin, so the text keeps the size a Retina snapshot of the Swift host has.
pub struct TextOverlay {
    font_system: FontSystem,
    swash_cache: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    renderer: TextRenderer,
    buffer: Buffer,
    scale: f32,
}

impl TextOverlay {
    /// Create a text renderer for an sRGB target with `sample_count`
    /// multisamples.
    pub fn new(context: &GpuContext, scale: f32, sample_count: u32) -> Self {
        let scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        let device = context.device();
        let mut font_system = FontSystem::new();
        let metrics = Metrics::new(FONT_SIZE * scale, LINE_HEIGHT * scale);
        let mut buffer = Buffer::new(&mut font_system, metrics);
        buffer.set_wrap(Wrap::None);

        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let mut atlas = TextAtlas::new(device, context.queue(), &cache, TARGET_FORMAT);
        let renderer = TextRenderer::new(
            &mut atlas,
            device,
            wgpu::MultisampleState {
                count: sample_count,
                ..Default::default()
            },
            None,
        );

        Self {
            font_system,
            swash_cache: SwashCache::new(),
            viewport,
            atlas,
            renderer,
            buffer,
            scale,
        }
    }

    /// Lay out `text` for a `width`×`height` frame.
    pub fn prepare(
        &mut self,
        context: &GpuContext,
        text: &str,
        width: u32,
        height: u32,
    ) -> Result<(), PrepareError> {
        self.viewport
            .update(context.queue(), Resolution { width, height });

        self.buffer
            .set_size(Some(width as f32), Some(height as f32));
        self.buffer.set_text(
            text,
            &Attrs::new().family(Family::Monospace).color(TEXT_COLOR),
            Shaping::Advanced,
            None,
        );
        self.buffer.shape_until_scroll(&mut self.font_system, false);

        let text_height = self
            .buffer
            .layout_runs()
            .map(|run| run.line_top + run.line_height)
            .fold(0.0f32, f32::max);
        let margin = MARGIN * self.scale;
        let top = (height as f32 - margin - text_height).max(0.0);

        let bounds = TextBounds {
            left: 0,
            top: 0,
            right: width as i32,
            bottom: height as i32,
        };
        self.renderer.prepare(
            context.device(),
            context.queue(),
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            [TextArea {
                buffer: &self.buffer,
                left: margin,
                top,
                scale: 1.0,
                bounds,
                default_color: TEXT_COLOR,
                custom_glyphs: &[],
            }],
            &mut self.swash_cache,
        )
    }

    /// Record the overlay draw. Must be called after [`Self::prepare`].
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if let Err(error) = self.renderer.render(&self.atlas, &self.viewport, pass) {
            log::warn!("cannot render the text overlay: {error}");
        }
    }

    /// Release atlas space after a frame.
    pub fn trim(&mut self) {
        self.atlas.trim();
    }
}
