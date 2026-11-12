//! Headless wgpu instance, adapter and device.

use std::error::Error;
use std::fmt;

/// A wgpu instance together with the adapter and device every renderer needs.
///
/// The context is headless: it never creates a surface. Windows and the
/// screensaver host add a surface to the same instance in I7–I11.
pub struct GpuContext {
    /// Kept alive for surface creation in later iterations.
    _instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    info: wgpu::AdapterInfo,
}

impl GpuContext {
    /// Create a context on the best available native GPU backend.
    pub fn new() -> Result<Self, GpuError> {
        Self::from_instance_descriptor(wgpu::InstanceDescriptor::new_without_display_handle())
    }

    /// Create a context on wgpu's `noop` backend.
    ///
    /// The noop backend accepts every descriptor and command but renders
    /// nothing, which lets CI validate the pipeline without a GPU.
    pub fn new_noop() -> Result<Self, GpuError> {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = wgpu::Backends::NOOP;
        descriptor.backend_options.noop.enable = true;
        Self::from_instance_descriptor(descriptor)
    }

    fn from_instance_descriptor(descriptor: wgpu::InstanceDescriptor) -> Result<Self, GpuError> {
        let instance = wgpu::Instance::new(descriptor);
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
            apply_limit_buckets: false,
        }))
        .map_err(GpuError::NoAdapter)?;
        let info = adapter.get_info();
        log::info!(
            "gpu adapter: {} ({:?}, {:?})",
            info.name,
            info.backend,
            info.device_type
        );
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .map_err(GpuError::NoDevice)?;
        Ok(Self {
            _instance: instance,
            device,
            queue,
            info,
        })
    }

    /// Description of the adapter in use.
    pub fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.info
    }

    /// The logical device.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// The submission queue.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
}

impl fmt::Debug for GpuContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GpuContext")
            .field("adapter", &self.info.name)
            .field("backend", &self.info.backend)
            .finish()
    }
}

/// A failure while creating or using the GPU device.
#[derive(Debug)]
pub enum GpuError {
    /// No adapter matches the requested backend and options.
    NoAdapter(wgpu::RequestAdapterError),
    /// The adapter refused to create a device.
    NoDevice(wgpu::RequestDeviceError),
    /// The requested frame size has a zero dimension.
    InvalidFrameSize {
        /// Requested width in pixels.
        width: u32,
        /// Requested height in pixels.
        height: u32,
    },
    /// The readback buffer could not be mapped.
    Readback(wgpu::BufferAsyncError),
    /// The mapped readback range is not accessible.
    MapRange(wgpu::MapRangeError),
    /// The device could not be polled to completion.
    Poll(wgpu::PollError),
    /// The readback callback never ran.
    ReadbackCancelled,
}

impl fmt::Display for GpuError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAdapter(error) => write!(formatter, "no GPU adapter: {error}"),
            Self::NoDevice(error) => write!(formatter, "cannot create a GPU device: {error}"),
            Self::InvalidFrameSize { width, height } => {
                write!(formatter, "invalid frame size {width}x{height}")
            }
            Self::Readback(error) => write!(formatter, "cannot read the frame back: {error}"),
            Self::MapRange(error) => write!(formatter, "cannot map the readback range: {error}"),
            Self::Poll(error) => write!(formatter, "GPU poll failed: {error}"),
            Self::ReadbackCancelled => write!(formatter, "the readback callback was cancelled"),
        }
    }
}

impl Error for GpuError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::NoAdapter(error) => Some(error),
            Self::NoDevice(error) => Some(error),
            Self::Readback(error) => Some(error),
            Self::MapRange(error) => Some(error),
            Self::Poll(error) => Some(error),
            Self::InvalidFrameSize { .. } | Self::ReadbackCancelled => None,
        }
    }
}
