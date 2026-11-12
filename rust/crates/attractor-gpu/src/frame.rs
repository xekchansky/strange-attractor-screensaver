//! One rendered frame: multisampled draw into an sRGB target.
//!
//! The same renderer serves two shells. A capture draws into its own texture
//! and reads the pixels back; the macOS host passes the swap chain drawable as
//! the resolve target and presents it. Both paths share the ribbon pipeline,
//! the text overlay and the multisample attachment.

use attractor_view::{Mat4, TrailGeometry};

use crate::context::{GpuContext, GpuError};
use crate::ribbon::RibbonRenderer;
#[cfg(feature = "text")]
use crate::text::TextOverlay;
use crate::{CLEAR_COLOR, SAMPLE_COUNT, TARGET_FORMAT};

/// The single-sampled texture and readback buffer of an offscreen frame.
struct Offscreen {
    resolve_texture: wgpu::Texture,
    resolve_view: wgpu::TextureView,
    readback: wgpu::Buffer,
    padded_row_bytes: u32,
}

/// Textures backing one frame size.
///
/// `multisample_view` is present when `sample_count` exceeds one and is then
/// the render attachment; the single-sampled resolve target is either the
/// offscreen texture or the swap chain view supplied per frame. With a count
/// of one the resolve target is the render attachment itself, which is what
/// the WebGL2 fallback and the noop backend need.
struct Attachment {
    multisample_view: Option<wgpu::TextureView>,
    offscreen: Option<Offscreen>,
}

impl Attachment {
    fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        sample_count: u32,
        offscreen: bool,
    ) -> Self {
        let extent = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let multisample_view = (sample_count > 1).then(|| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("frame multisample target"),
                    size: extent,
                    mip_level_count: 1,
                    sample_count,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        });

        let offscreen = offscreen.then(|| {
            let resolve_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("frame resolve target"),
                size: extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let resolve_view = resolve_texture.create_view(&wgpu::TextureViewDescriptor::default());
            let padded_row_bytes = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
                * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
            let readback = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("frame readback"),
                size: u64::from(padded_row_bytes) * u64::from(height),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            Offscreen {
                resolve_texture,
                resolve_view,
                readback,
                padded_row_bytes,
            }
        });

        Self {
            multisample_view,
            offscreen,
        }
    }

    /// The attachment pair of one pass: draw target and optional resolve.
    fn draw_targets<'a>(
        &'a self,
        external: &'a wgpu::TextureView,
    ) -> (&'a wgpu::TextureView, Option<&'a wgpu::TextureView>) {
        match &self.multisample_view {
            Some(view) => (view, Some(external)),
            None => (external, None),
        }
    }
}

/// Renders complete frames of one size into an sRGB target.
///
/// [`FrameRenderer::new`] and [`FrameRenderer::with_sample_count`] own an
/// offscreen target and read frames back; [`FrameRenderer::for_surface`]
/// builds the same pipelines for a swap chain format and draws into the view
/// the caller passes to [`FrameRenderer::render_to_view`]. Textures are created
/// per frame size and survive between frames; [`FrameRenderer::resize`]
/// replaces them when the surface changes.
pub struct FrameRenderer {
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    sample_count: u32,
    overlay_scale: f32,
    attachment: Attachment,
    ribbon: RibbonRenderer,
    #[cfg(feature = "text")]
    text: Option<TextOverlay>,
}

impl FrameRenderer {
    /// Create an offscreen renderer for `width`×`height` pixels.
    ///
    /// `overlay_scale` multiplies the overlay font and margin, so a capture at
    /// `--capture-scale 2` keeps the same apparent text size as the Swift
    /// Retina snapshot.
    pub fn new(
        context: &GpuContext,
        width: u32,
        height: u32,
        overlay_scale: f32,
    ) -> Result<Self, GpuError> {
        Self::with_sample_count(context, width, height, overlay_scale, SAMPLE_COUNT)
    }

    /// Create an offscreen renderer with an explicit multisample count.
    ///
    /// The WebGL2 fallback cannot resolve multisampled targets and passes one
    /// here in I9; one also makes the CI noop backend usable.
    pub fn with_sample_count(
        context: &GpuContext,
        width: u32,
        height: u32,
        overlay_scale: f32,
        sample_count: u32,
    ) -> Result<Self, GpuError> {
        Self::build(
            context,
            TARGET_FORMAT,
            width,
            height,
            overlay_scale,
            sample_count,
            true,
        )
    }

    /// Create a renderer for a presentation surface of `format`.
    ///
    /// The renderer owns no resolve texture and no readback buffer: each frame
    /// is resolved into the swap chain view supplied to
    /// [`FrameRenderer::render_to_view`].
    pub fn for_surface(
        context: &GpuContext,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        overlay_scale: f32,
        sample_count: u32,
    ) -> Result<Self, GpuError> {
        Self::build(
            context,
            format,
            width,
            height,
            overlay_scale,
            sample_count,
            false,
        )
    }

    fn build(
        context: &GpuContext,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        overlay_scale: f32,
        sample_count: u32,
        offscreen: bool,
    ) -> Result<Self, GpuError> {
        if width == 0 || height == 0 {
            return Err(GpuError::InvalidFrameSize { width, height });
        }
        let sample_count = sample_count.max(1);
        let overlay_scale = sanitized_scale(overlay_scale);
        #[cfg(feature = "text")]
        let text = Some(TextOverlay::new(
            context,
            format,
            overlay_scale,
            sample_count,
        ));
        Ok(Self {
            width,
            height,
            format,
            sample_count,
            overlay_scale,
            attachment: Attachment::new(
                context.device(),
                width,
                height,
                format,
                sample_count,
                offscreen,
            ),
            ribbon: RibbonRenderer::new(context, format, sample_count),
            #[cfg(feature = "text")]
            text,
        })
    }

    /// Frame size in pixels.
    pub const fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Target format of the pipelines.
    pub const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Multisample count of the target.
    pub const fn sample_count(&self) -> u32 {
        self.sample_count
    }

    /// Retarget the renderer at a new frame size.
    ///
    /// The pipelines and the ribbon buffers survive; only the attachment
    /// textures and the readback buffer are recreated. A non-finite or
    /// non-positive `overlay_scale` falls back to one. The call is a no-op
    /// when nothing changed.
    pub fn resize(
        &mut self,
        context: &GpuContext,
        width: u32,
        height: u32,
        overlay_scale: f32,
    ) -> Result<(), GpuError> {
        if width == 0 || height == 0 {
            return Err(GpuError::InvalidFrameSize { width, height });
        }
        let overlay_scale = sanitized_scale(overlay_scale);
        if width == self.width && height == self.height && overlay_scale == self.overlay_scale {
            return Ok(());
        }
        self.attachment = Attachment::new(
            context.device(),
            width,
            height,
            self.format,
            self.sample_count,
            self.attachment.offscreen.is_some(),
        );
        self.width = width;
        self.height = height;
        #[cfg(feature = "text")]
        if overlay_scale != self.overlay_scale {
            self.text = Some(TextOverlay::new(
                context,
                self.format,
                overlay_scale,
                self.sample_count,
            ));
        }
        self.overlay_scale = overlay_scale;
        Ok(())
    }

    /// Render one frame and return tightly packed RGBA8 rows.
    ///
    /// Only renderers created by [`FrameRenderer::new`] or
    /// [`FrameRenderer::with_sample_count`] own a readback buffer; a surface
    /// renderer returns [`GpuError::NoReadbackTarget`].
    pub fn render(
        &mut self,
        context: &GpuContext,
        geometry: &TrailGeometry,
        view_projection: &Mat4,
        overlay: Option<&str>,
    ) -> Result<Vec<u8>, GpuError> {
        let resolve_view = self
            .attachment
            .offscreen
            .as_ref()
            .ok_or(GpuError::NoReadbackTarget)?
            .resolve_view
            .clone();
        self.render_to_view(context, &resolve_view, geometry, view_projection, overlay)?;

        let offscreen = self
            .attachment
            .offscreen
            .as_ref()
            .ok_or(GpuError::NoReadbackTarget)?;
        let slice = offscreen.readback.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        context
            .device()
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(GpuError::Poll)?;
        receiver
            .recv()
            .map_err(|_| GpuError::ReadbackCancelled)?
            .map_err(GpuError::Readback)?;

        let mapped = slice.get_mapped_range().map_err(GpuError::MapRange)?;
        let row_bytes = (self.width * 4) as usize;
        let mut pixels = Vec::with_capacity(row_bytes * self.height as usize);
        for row in 0..self.height as usize {
            let start = row * offscreen.padded_row_bytes as usize;
            pixels.extend_from_slice(&mapped[start..start + row_bytes]);
        }
        drop(mapped);
        offscreen.readback.unmap();
        Ok(pixels)
    }

    /// Render one frame into `target`, which must have the renderer's format
    /// and size.
    ///
    /// `overlay` draws the corner text on top of the ribbon when the `text`
    /// feature is enabled and a font was found. Nothing is read back; a
    /// surface renderer presents the target after the call.
    pub fn render_to_view(
        &mut self,
        context: &GpuContext,
        target: &wgpu::TextureView,
        geometry: &TrailGeometry,
        view_projection: &Mat4,
        overlay: Option<&str>,
    ) -> Result<(), GpuError> {
        let device = context.device();
        let queue = context.queue();
        self.ribbon.upload(context, geometry);
        self.ribbon.set_view_projection(context, view_projection);

        #[cfg(feature = "text")]
        if let (Some(text), Some(overlay)) = (self.text.as_mut(), overlay)
            && let Err(error) = text.prepare(context, overlay, self.width, self.height)
        {
            log::warn!("cannot lay out the text overlay: {error}");
        }

        let (draw_view, resolve_view) = self.attachment.draw_targets(target);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("frame encoder"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ribbon and overlay pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: draw_view,
                    depth_slice: None,
                    resolve_target: resolve_view,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            self.ribbon.draw(&mut pass);
            #[cfg(feature = "text")]
            if let Some(text) = &self.text {
                text.draw(&mut pass);
            }
        }
        #[cfg(feature = "text")]
        if let Some(text) = self.text.as_mut() {
            text.trim();
        }

        // The offscreen path copies the resolve texture after the pass; the
        // caller of the surface path presents the swap chain image instead.
        let mut commands = vec![encoder.finish()];
        if let Some(offscreen) = &self.attachment.offscreen {
            let mut copy = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame copy encoder"),
            });
            copy.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &offscreen.resolve_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &offscreen.readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(offscreen.padded_row_bytes),
                        rows_per_image: Some(self.height),
                    },
                },
                wgpu::Extent3d {
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
            );
            commands.push(copy.finish());
        }
        queue.submit(commands);
        Ok(())
    }
}

/// Overlay scale the text renderer can lay out.
fn sanitized_scale(scale: f32) -> f32 {
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}
