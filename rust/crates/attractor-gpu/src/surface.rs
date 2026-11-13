//! A presentation surface created from a native platform handle.
//!
//! The macOS `.saver` owns a layer-backed `NSView` and passes its
//! `CAMetalLayer` to the player; the Windows `.scr` passes its `HWND`. wgpu
//! wraps either handle in a swap chain, so the frame is presented straight to
//! the screen instead of being read back. The Apple and Windows constructors
//! share one type, so every shell presents through the same code; I10 adds the
//! X11 and Wayland handles, I11 the Android wallpaper window.

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

/// The handle a surface was created from.
///
/// The host guarantees that the native handle outlives the target; it is only
/// used again when wgpu reports the surface lost. Platforms without a
/// constructor contribute no variants, which keeps the rest of the type
/// portable.
#[derive(Clone)]
enum SurfaceSource {
    /// The `CAMetalLayer` the surface was created from.
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    CoreAnimationLayer(*mut std::ffi::c_void),
    /// The `HWND` (and its `HINSTANCE`) the surface was created from.
    #[cfg(target_os = "windows")]
    Win32Window {
        /// Window handle the swap chain presents into.
        hwnd: *mut std::ffi::c_void,
        /// Module instance the window belongs to; may be null.
        hinstance: *mut std::ffi::c_void,
    },
    /// Raw X11 or Wayland handles supplied by the Linux shell.
    #[cfg(target_os = "linux")]
    RawHandle {
        /// The window the swap chain presents into.
        window: wgpu::rwh::RawWindowHandle,
        /// The display the window belongs to; the Vulkan backend needs it.
        display: Option<wgpu::rwh::RawDisplayHandle>,
    },
    /// The `ANativeWindow` behind the wallpaper's `Surface`.
    #[cfg(target_os = "android")]
    AndroidWindow(*mut std::ffi::c_void),
    /// The `<canvas>` the browser surface was created from.
    #[cfg(target_arch = "wasm32")]
    Canvas(web_sys::HtmlCanvasElement),
}

impl SurfaceSource {
    /// The wgpu target for this native handle.
    ///
    /// Taking the source by value keeps the match exhaustive on targets that
    /// contribute no variants (a reference is always inhabited, an empty enum
    /// is not).
    #[cfg(not(target_arch = "wasm32"))]
    fn unsafe_target(self) -> wgpu::SurfaceTargetUnsafe {
        match self {
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            Self::CoreAnimationLayer(layer) => wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(layer),
            #[cfg(target_os = "windows")]
            Self::Win32Window { hwnd, hinstance } => win32_target(hwnd, hinstance),
            #[cfg(target_os = "linux")]
            Self::RawHandle { window, display } => wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_window_handle: window,
                raw_display_handle: display,
            },
            #[cfg(target_os = "android")]
            Self::AndroidWindow(window) => android_target(window),
        }
    }
}

/// Create the swap chain for `source`.
///
/// A browser canvas is owned by the surface, so the safe wgpu constructor
/// applies; native layers and windows are borrowed handles and go through the
/// unsafe constructor under the contract stated on each public constructor.
// A target without a surface constructor contributes no `SurfaceSource`
// variants, which makes this body unreachable there by construction.
#[cfg_attr(
    not(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "windows",
        target_os = "linux",
        target_os = "android",
        target_arch = "wasm32"
    )),
    allow(unreachable_code)
)]
fn create_surface(
    instance: &wgpu::Instance,
    source: &SurfaceSource,
) -> Result<wgpu::Surface<'static>, GpuError> {
    #[cfg(target_arch = "wasm32")]
    let surface = match source {
        SurfaceSource::Canvas(canvas) => {
            instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
        }
    };
    #[cfg(not(target_arch = "wasm32"))]
    // SAFETY: the public constructors require the handle to outlive the target.
    let surface = unsafe { instance.create_surface_unsafe(source.clone().unsafe_target()) };
    surface.map_err(GpuError::Surface)
}

/// Wrap a Win32 window handle in wgpu's raw-handle target.
///
/// `hwnd` must be non-null; the constructor checks that before calling.
#[cfg(target_os = "windows")]
fn win32_target(
    hwnd: *mut std::ffi::c_void,
    hinstance: *mut std::ffi::c_void,
) -> wgpu::SurfaceTargetUnsafe {
    use std::num::NonZeroIsize;
    use wgpu::rwh::{RawDisplayHandle, RawWindowHandle, Win32WindowHandle, WindowsDisplayHandle};

    // SAFETY: the caller of the public constructor guarantees a live HWND.
    let hwnd = NonZeroIsize::new(hwnd as isize).expect("the window handle is non-null");
    let mut window = Win32WindowHandle::new(hwnd);
    window.hinstance = NonZeroIsize::new(hinstance as isize);
    wgpu::SurfaceTargetUnsafe::RawHandle {
        raw_display_handle: Some(RawDisplayHandle::Windows(WindowsDisplayHandle::new())),
        raw_window_handle: RawWindowHandle::Win32(window),
    }
}

/// Wrap an Android native window in wgpu's raw-handle target.
///
/// The window comes from `ANativeWindow_fromSurface`; the constructor checks
/// it is not null before calling.
#[cfg(target_os = "android")]
fn android_target(window: *mut std::ffi::c_void) -> wgpu::SurfaceTargetUnsafe {
    use wgpu::rwh::{
        AndroidDisplayHandle, AndroidNdkWindowHandle, RawDisplayHandle, RawWindowHandle,
    };

    // SAFETY: the caller of the public constructor guarantees a live window.
    let window = std::ptr::NonNull::new(window).expect("the native window is non-null");
    wgpu::SurfaceTargetUnsafe::RawHandle {
        raw_display_handle: Some(RawDisplayHandle::Android(AndroidDisplayHandle::new())),
        raw_window_handle: RawWindowHandle::AndroidNdk(AndroidNdkWindowHandle::new(window)),
    }
}

/// A swap chain over one native layer.
///
/// The player owns one target per surface. The target keeps the raw handle so
/// a lost surface can be recreated, but it never touches the handle itself:
/// presentation goes through wgpu.
pub struct SurfaceTarget {
    source: SurfaceSource,
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
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    pub unsafe fn from_metal_layer(
        context: &GpuContext,
        layer: *mut std::ffi::c_void,
        width: u32,
        height: u32,
    ) -> Result<Self, GpuError> {
        if width == 0 || height == 0 {
            return Err(GpuError::InvalidFrameSize { width, height });
        }
        let source = SurfaceSource::CoreAnimationLayer(layer);
        let surface = create_surface(context.instance(), &source)?;
        let config = surface_configuration(&surface, context.adapter(), width, height)?;
        surface.configure(context.device(), &config);
        Ok(Self {
            source,
            surface,
            config,
        })
    }

    /// Wrap the Win32 window `hwnd` in a swap chain sized `width`×`height`
    /// pixels.
    ///
    /// The `.scr` shell calls this once per monitor window. `hinstance` is the
    /// module handle the window class was registered with; pass null when it
    /// is unknown.
    ///
    /// # Safety
    ///
    /// `hwnd` must be a live window that outlives the returned target.
    #[cfg(target_os = "windows")]
    pub unsafe fn from_window_handle(
        context: &GpuContext,
        hwnd: *mut std::ffi::c_void,
        hinstance: *mut std::ffi::c_void,
        width: u32,
        height: u32,
    ) -> Result<Self, GpuError> {
        if hwnd.is_null() {
            return Err(GpuError::InvalidWindowHandle);
        }
        if width == 0 || height == 0 {
            return Err(GpuError::InvalidFrameSize { width, height });
        }
        let source = SurfaceSource::Win32Window { hwnd, hinstance };
        let surface = create_surface(context.instance(), &source)?;
        let config = surface_configuration(&surface, context.adapter(), width, height)?;
        surface.configure(context.device(), &config);
        Ok(Self {
            source,
            surface,
            config,
        })
    }

    /// Wrap a raw X11 or Wayland window in a swap chain sized `width`×`height`
    /// pixels.
    ///
    /// The Linux shells own their windows through different libraries (raw
    /// Xlib for the xscreensaver hack, winit for the fullscreen runner), so
    /// they hand over `raw-window-handle` values instead of a platform type.
    /// The Vulkan backend needs the display handle as well; an X11 shell
    /// passes its `Display*` or `xcb_connection_t*`, a Wayland shell its
    /// `wl_display*`.
    ///
    /// # Safety
    ///
    /// The window and the display a handle points to must outlive the
    /// returned target.
    #[cfg(target_os = "linux")]
    pub unsafe fn from_raw_handles(
        context: &GpuContext,
        window: wgpu::rwh::RawWindowHandle,
        display: Option<wgpu::rwh::RawDisplayHandle>,
        width: u32,
        height: u32,
    ) -> Result<Self, GpuError> {
        if width == 0 || height == 0 {
            return Err(GpuError::InvalidFrameSize { width, height });
        }
        let source = SurfaceSource::RawHandle { window, display };
        let surface = create_surface(context.instance(), &source)?;
        let config = surface_configuration(&surface, context.adapter(), width, height)?;
        surface.configure(context.device(), &config);
        Ok(Self {
            source,
            surface,
            config,
        })
    }

    /// Create the GPU context and the swap chain over the `ANativeWindow`
    /// behind an Android wallpaper surface.
    ///
    /// The surface must exist before the adapter is requested: the GLES
    /// backend binds its adapter to the window, and a second adapter would not
    /// be able to present into it. This is why the constructor returns both
    /// halves, like the browser bootstrap does.
    ///
    /// The device asks for `adapter.limits()` rather than the desktop
    /// defaults, because a GLES adapter reports smaller limits than the
    /// defaults request.
    ///
    /// # Safety
    ///
    /// `window` must be a live `ANativeWindow` reference that outlives the
    /// returned target; release it only after the target is dropped.
    #[cfg(target_os = "android")]
    pub unsafe fn create_android(
        window: *mut std::ffi::c_void,
        width: u32,
        height: u32,
    ) -> Result<(GpuContext, Self), GpuError> {
        if window.is_null() {
            return Err(GpuError::InvalidWindowHandle);
        }
        if width == 0 || height == 0 {
            return Err(GpuError::InvalidFrameSize { width, height });
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let source = SurfaceSource::AndroidWindow(window);
        let surface = create_surface(&instance, &source)?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
            apply_limit_buckets: false,
        }))
        .map_err(GpuError::NoAdapter)?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_limits: adapter.limits(),
            ..Default::default()
        }))
        .map_err(GpuError::NoDevice)?;
        let config = surface_configuration(&surface, &adapter, width, height)?;
        surface.configure(&device, &config);
        let context = GpuContext::from_parts(instance, adapter, device, queue);
        Ok((
            context,
            Self {
                source,
                surface,
                config,
            },
        ))
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
        let surface = create_surface(context.instance(), &self.source)?;
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

#[cfg(target_arch = "wasm32")]
impl SurfaceTarget {
    /// Create the browser GPU context and the canvas surface together.
    ///
    /// The adapter is requested with the surface attached: the WebGL2 backend
    /// binds the adapter to the canvas context, so creating the surface after
    /// the adapter is not equivalent. Unless `force_webgl2` is set, the
    /// instance prefers WebGPU and falls back to WebGL2 when the browser does
    /// not expose a working `navigator.gpu` adapter. The canvas is owned by
    /// the surface, so unlike the native constructors this one is safe.
    pub async fn create_web(
        canvas: web_sys::HtmlCanvasElement,
        width: u32,
        height: u32,
        force_webgl2: bool,
    ) -> Result<(GpuContext, Self), GpuError> {
        if width == 0 || height == 0 {
            return Err(GpuError::InvalidFrameSize { width, height });
        }
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        let instance = if force_webgl2 {
            descriptor.backends = wgpu::Backends::GL;
            wgpu::Instance::new(descriptor)
        } else {
            wgpu::util::new_instance_with_webgpu_detection(descriptor).await
        };
        let source = SurfaceSource::Canvas(canvas);
        let surface = create_surface(&instance, &source)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
                apply_limit_buckets: false,
            })
            .await
            .map_err(GpuError::NoAdapter)?;
        // Ask for what the adapter reports rather than the desktop defaults.
        // A WebGL2 adapter has no compute support and reports zero compute
        // limits, while the default limits request 65535 compute workgroups,
        // so the default descriptor fails the device request outright.
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: adapter.limits(),
                ..Default::default()
            })
            .await
            .map_err(GpuError::NoDevice)?;
        let config = surface_configuration(&surface, &adapter, width, height)?;
        surface.configure(&device, &config);
        let context = GpuContext::from_parts(instance, adapter, device, queue);
        Ok((
            context,
            Self {
                source,
                surface,
                config,
            },
        ))
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

#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "windows",
    target_os = "linux",
    target_os = "android",
    target_arch = "wasm32"
))]
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

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use std::num::NonZeroU32;

    use wgpu::rwh::{RawDisplayHandle, RawWindowHandle, XcbDisplayHandle, XcbWindowHandle};

    use super::*;

    #[test]
    fn a_zero_frame_size_is_rejected_before_the_handles_are_used() {
        let context = GpuContext::new_noop().expect("the noop backend always exists");
        let window = XcbWindowHandle::new(NonZeroU32::new(1).expect("one is not zero"));
        let display = XcbDisplayHandle::new(None, 0);
        let result = unsafe {
            SurfaceTarget::from_raw_handles(
                &context,
                RawWindowHandle::Xcb(window),
                Some(RawDisplayHandle::Xcb(display)),
                0,
                480,
            )
        };
        assert!(matches!(
            result,
            Err(GpuError::InvalidFrameSize { width: 0, .. })
        ));
    }
}
