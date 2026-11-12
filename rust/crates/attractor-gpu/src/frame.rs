//! One offscreen frame: multisampled draw, resolve, read back RGBA8.

use attractor_view::{Mat4, TrailGeometry};

use crate::context::{GpuContext, GpuError};
use crate::ribbon::RibbonRenderer;
#[cfg(feature = "text")]
use crate::text::TextOverlay;
use crate::{CLEAR_COLOR, SAMPLE_COUNT, TARGET_FORMAT};

/// Renders complete frames of one size into an sRGB target and reads them back.
///
/// The target is multisampled and resolved to a single-sampled texture before
/// the copy, mirroring the Swift host's `.multisampling4X` snapshots. Both
/// textures and the readback buffer live for the renderer's lifetime, so a
/// recording loop allocates nothing per frame except the returned pixel
/// vector.
pub struct FrameRenderer {
    width: u32,
    height: u32,
    sample_count: u32,
    multisample_view: wgpu::TextureView,
    resolve_texture: wgpu::Texture,
    resolve_view: wgpu::TextureView,
    readback: wgpu::Buffer,
    padded_row_bytes: u32,
    ribbon: RibbonRenderer,
    #[cfg(feature = "text")]
    text: Option<TextOverlay>,
}

impl FrameRenderer {
    /// Create a renderer for `width`×`height` pixels.
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

    /// Create a renderer with an explicit multisample count.
    ///
    /// The WebGL2 fallback cannot resolve multisampled targets and will pass
    /// one here in I9; one also makes the CI noop backend usable.
    pub fn with_sample_count(
        context: &GpuContext,
        width: u32,
        height: u32,
        overlay_scale: f32,
        sample_count: u32,
    ) -> Result<Self, GpuError> {
        if width == 0 || height == 0 {
            return Err(GpuError::InvalidFrameSize { width, height });
        }
        let device = context.device();
        let extent = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let multisample = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("frame multisample target"),
            size: extent,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: TARGET_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let resolve = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("frame resolve target"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TARGET_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let padded_row_bytes = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame readback"),
            size: u64::from(padded_row_bytes) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        #[cfg(feature = "text")]
        let text = Some(TextOverlay::new(context, overlay_scale, sample_count));

        Ok(Self {
            width,
            height,
            sample_count,
            multisample_view: multisample.create_view(&wgpu::TextureViewDescriptor::default()),
            resolve_view: resolve.create_view(&wgpu::TextureViewDescriptor::default()),
            resolve_texture: resolve,
            readback,
            padded_row_bytes,
            ribbon: RibbonRenderer::new(context, TARGET_FORMAT, sample_count),
            #[cfg(feature = "text")]
            text,
        })
    }

    /// Frame size in pixels.
    pub const fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Multisample count of the target.
    pub const fn sample_count(&self) -> u32 {
        self.sample_count
    }

    /// Render one frame and return tightly packed RGBA8 rows.
    ///
    /// `overlay` draws the corner text on top of the ribbon when the `text`
    /// feature is enabled and a font was found.
    pub fn render(
        &mut self,
        context: &GpuContext,
        geometry: &TrailGeometry,
        view_projection: &Mat4,
        overlay: Option<&str>,
    ) -> Result<Vec<u8>, GpuError> {
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

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("frame encoder"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ribbon and overlay pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.multisample_view,
                    depth_slice: None,
                    resolve_target: Some(&self.resolve_view),
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
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.resolve_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.padded_row_bytes),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);

        let slice = self.readback.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        device
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
            let start = row * self.padded_row_bytes as usize;
            pixels.extend_from_slice(&mapped[start..start + row_bytes]);
        }
        drop(mapped);
        self.readback.unmap();
        Ok(pixels)
    }
}
