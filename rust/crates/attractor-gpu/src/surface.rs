//! A presentation surface created from a native `CAMetalLayer`.
//!
//! The macOS `.saver` owns a layer-backed `NSView` and passes its
//! `CAMetalLayer` to the player. wgpu wraps that layer in a swap chain, so the
//! frame is presented straight to the screen instead of being read back.

use std::ffi::c_void;

use crate::context::{GpuContext, GpuError};

/// An sRGB format the pipeline prefers for presentation.
const PREFERRED_FORMATS: [wgpu::TextureFormat; 2] = [
    wgpu::TextureFormat::Bgra8UnormSrgb,
    wgpu::TextureFormat::Rgba8UnormSrgb,
];

/// The format the swap chain uses, or `None` when the surface is unusable.
///
/// A surface offers formats in preference order. The renderer needs an sRGB
/// target so the linear ribbon colors are encoded on write, exactly like the
/// offscreen `TARGET_FORMAT`; a platform that only offers linear formats gets
/// its first one and the picture is slightly brighter than the capture.
pub fn choose_surface_format(formats: &[wgpu::TextureFormat]) -> Option<wgpu::TextureFormat> {
    PREFERRED_FORMATS
        .iter()
        .find(|format| formats.contains(format))
        .copied()
        .or_else(|| formats.iter().copied().find(wgpu::TextureFormat::is_srgb))
        .or_else(|| formats.first().copied())
}

/// A swap chain over one native layer.
///
/// The player owns one target per surface. The target keeps the raw layer
/// pointer so a lost surface can be recreated, but it never touches the layer
/// itself: presentation goes through wgpu.
pub struct SurfaceTarget {
    /// The `CAMetalLayer` the surface was created from.
    ///
    /// The host guarantees that the layer outlives the target; the pointer is
    /// only used again when wgpu reports the surface lost.
    layer: *mut c_void,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
}

impl SurfaceTarget {
    /// Wrap `layer` in a swap chain sized `width`×`height` pixels.
    ///
    /// # Safety
    ///
    /// `layer` must be a live `CAMetalLayer` that outlives the returned
    /// target.
    pub unsafe fn from_metal_layer(
        context: &GpuContext,
        layer: *mut c_void,
        width: u32,
        height: u32,
    ) -> Result<Self, GpuError> {
        if width == 0 || height == 0 {
            return Err(GpuError::InvalidFrameSize { width, height });
        }
        // SAFETY: the caller guarantees the layer outlives the surface.
        let surface = unsafe {
            context
                .instance()
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(layer))
        }
        .map_err(GpuError::Surface)?;
        let config = surface_configuration(&surface, context.adapter(), width, height)?;
        surface.configure(context.device(), &config);
        Ok(Self {
            layer,
            surface,
            config,
        })
    }

    /// The swap chain format the renderer builds its pipelines for.
    pub const fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    /// Current swap chain size in pixels.
    pub const fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    /// Retarget the swap chain at a new frame size.
    ///
    /// The call is a no-op when the size did not change, so a host may pass
    /// the player size on every frame.
    pub fn resize(
        &mut self,
        context: &GpuContext,
        width: u32,
        height: u32,
    ) -> Result<(), GpuError> {
        if width == 0 || height == 0 {
            return Err(GpuError::InvalidFrameSize { width, height });
        }
        if self.config.width == width && self.config.height == height {
            return Ok(());
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(context.device(), &self.config);
        Ok(())
    }

    /// Acquire the next drawable.
    ///
    /// `Ok(None)` means the frame should be skipped: the drawable timed out or
    /// the layer is occluded. An outdated or lost surface is reconfigured once
    /// before the frame is retried, which is what a display-mode or window
    /// change looks like from wgpu.
    pub fn acquire(
        &mut self,
        context: &GpuContext,
    ) -> Result<Option<wgpu::SurfaceTexture>, GpuError> {
        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => Ok(Some(frame)),
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                Ok(None)
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.reconfigure(context)?;
                match self.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(frame)
                    | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => Ok(Some(frame)),
                    wgpu::CurrentSurfaceTexture::Timeout
                    | wgpu::CurrentSurfaceTexture::Occluded => Ok(None),
                    wgpu::CurrentSurfaceTexture::Outdated => {
                        Err(GpuError::SurfaceAcquire("the surface is still outdated"))
                    }
                    wgpu::CurrentSurfaceTexture::Lost => self.recreate(context),
                    wgpu::CurrentSurfaceTexture::Validation => {
                        Err(GpuError::SurfaceAcquire("surface validation failed"))
                    }
                }
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                Err(GpuError::SurfaceAcquire("surface validation failed"))
            }
        }
    }

    fn reconfigure(&mut self, context: &GpuContext) -> Result<(), GpuError> {
        self.surface.configure(context.device(), &self.config);
        Ok(())
    }

    /// Rebuild the wgpu surface when the swap chain reports it lost.
    fn recreate(&mut self, context: &GpuContext) -> Result<Option<wgpu::SurfaceTexture>, GpuError> {
        // SAFETY: the host keeps the layer alive for the target's lifetime.
        let surface = unsafe {
            context
                .instance()
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(self.layer))
        }
        .map_err(GpuError::Surface)?;
        surface.configure(context.device(), &self.config);
        self.surface = surface;
        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => Ok(Some(frame)),
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                Ok(None)
            }
            wgpu::CurrentSurfaceTexture::Outdated
            | wgpu::CurrentSurfaceTexture::Lost
            | wgpu::CurrentSurfaceTexture::Validation => Err(GpuError::SurfaceAcquire(
                "the recreated surface is unusable",
            )),
        }
    }
}

impl std::fmt::Debug for SurfaceTarget {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SurfaceTarget")
            .field("format", &self.config.format)
            .field("size", &(self.config.width, self.config.height))
            .finish_non_exhaustive()
    }
}

fn surface_configuration(
    surface: &wgpu::Surface<'_>,
    adapter: &wgpu::Adapter,
    width: u32,
    height: u32,
) -> Result<wgpu::SurfaceConfiguration, GpuError> {
    let capabilities = surface.get_capabilities(adapter);
    let format =
        choose_surface_format(&capabilities.formats).ok_or(GpuError::UnsupportedSurface)?;
    let present_mode = if capabilities
        .present_modes
        .contains(&wgpu::PresentMode::Fifo)
    {
        wgpu::PresentMode::Fifo
    } else {
        capabilities
            .present_modes
            .first()
            .copied()
            .unwrap_or(wgpu::PresentMode::Fifo)
    };
    let alpha_mode = if capabilities
        .alpha_modes
        .contains(&wgpu::CompositeAlphaMode::Opaque)
    {
        wgpu::CompositeAlphaMode::Opaque
    } else {
        capabilities
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto)
    };
    Ok(wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        color_space: wgpu::SurfaceColorSpace::Auto,
        width,
        height,
        desired_maximum_frame_latency: 2,
        present_mode,
        alpha_mode,
        view_formats: Vec::new(),
    })
}
