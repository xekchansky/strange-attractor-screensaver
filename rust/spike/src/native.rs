//! Native offscreen render: one frame, one PNG.

use std::path::Path;

use crate::{CLEAR, build_scene, camera_matrix, gpu::Renderer};

pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

pub struct Offscreen {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub renderer: Renderer,
    pub width: u32,
    pub height: u32,
    pub scene_radius: f32,
}

pub fn init(width: u32, height: u32) -> Offscreen {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
        apply_limit_buckets: false,
    }))
    .expect("no adapter");

    let info = adapter.get_info();
    log::info!(
        "adapter: {} ({:?}, {:?})",
        info.name,
        info.backend,
        info.device_type
    );

    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("no device");

    let (vertices, scene_radius) = build_scene();
    let renderer = Renderer::new(
        &device,
        &queue,
        FORMAT,
        &vertices,
        camera_matrix(width, height, 0.0, scene_radius),
    );

    Offscreen {
        device,
        queue,
        renderer,
        width,
        height,
        scene_radius,
    }
}

impl Offscreen {
    /// Renders one frame and returns tightly packed RGBA8 rows.
    pub fn render(&self, seconds: f32) -> Vec<u8> {
        let device = &self.device;
        let queue = &self.queue;
        self.renderer.update_camera(
            queue,
            camera_matrix(self.width, self.height, seconds, self.scene_radius),
        );

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen target"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let padded_row = (self.width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: padded_row as u64 * self.height as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("frame"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ribbon pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(CLEAR),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            self.renderer.draw(&mut pass);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row),
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

        let slice = readback.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll failed");
        receiver.recv().expect("map callback").expect("map failed");

        let mapped = slice.get_mapped_range().expect("map range");
        let row_bytes = (self.width * 4) as usize;
        let mut pixels = Vec::with_capacity(row_bytes * self.height as usize);
        for row in 0..self.height as usize {
            let start = row * padded_row as usize;
            pixels.extend_from_slice(&mapped[start..start + row_bytes]);
        }
        drop(mapped);
        readback.unmap();
        pixels
    }
}

/// Writes RGBA8 pixels as a PNG file.
pub fn write_png(path: &Path, width: u32, height: u32, pixels: &[u8]) -> std::io::Result<()> {
    let file = std::fs::File::create(path)?;
    let writer = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(std::io::Error::other)?;
    writer
        .write_image_data(pixels)
        .map_err(std::io::Error::other)?;
    writer.finish().map_err(std::io::Error::other)?;
    Ok(())
}

/// Encodes one linear channel the way an `Rgba8UnormSrgb` target stores it.
fn srgb_byte(linear: f64) -> u8 {
    let encoded = if linear <= 0.003_130_8 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round().clamp(0.0, 255.0) as u8
}

/// The clear color as it appears in the sRGB-encoded readback.
pub fn clear_rgb8() -> [u8; 3] {
    [srgb_byte(CLEAR.r), srgb_byte(CLEAR.g), srgb_byte(CLEAR.b)]
}

/// Counts pixels that differ from the clear color, as a cheap "not blank" check.
pub fn lit_pixels(pixels: &[u8]) -> usize {
    let clear = clear_rgb8();
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| {
            p[0].abs_diff(clear[0]) > 4
                || p[1].abs_diff(clear[1]) > 4
                || p[2].abs_diff(clear[2]) > 4
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srgb_encoding_matches_known_values() {
        assert_eq!(srgb_byte(0.0), 0);
        assert_eq!(srgb_byte(1.0), 255);
        assert_eq!(srgb_byte(0.015), 33);
        assert_eq!(srgb_byte(0.030), 48);
    }

    #[test]
    fn background_of_a_fresh_frame_is_not_lit() {
        let [r, g, b] = clear_rgb8();
        let pixels: Vec<u8> = std::iter::repeat_n([r, g, b, 255], 4).flatten().collect();
        assert_eq!(lit_pixels(&pixels), 0);
    }

    #[test]
    fn a_pixel_that_differs_from_the_background_is_lit() {
        let [r, g, b] = clear_rgb8();
        let mut pixels: Vec<u8> = std::iter::repeat_n([r, g, b, 255], 4).flatten().collect();
        pixels[0] = 255;
        pixels[1] = 0;
        pixels[2] = 0;
        assert_eq!(lit_pixels(&pixels), 1);
    }
}
