//! Browser entry point: WebGPU with a WebGL fallback, rendering to a canvas.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use crate::{build_scene, camera_matrix, gpu::Renderer};

/// The animation-frame callback that drives the render loop.
type FrameCallback = Closure<dyn FnMut(f64)>;

struct WebState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: Renderer,
    config: wgpu::SurfaceConfiguration,
    scene_radius: f32,
}

impl WebState {
    fn render(&mut self, seconds: f32) {
        self.renderer.update_camera(
            &self.queue,
            camera_matrix(
                self.config.width,
                self.config.height,
                seconds,
                self.scene_radius,
            ),
        );
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                match self.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(frame)
                    | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
                    other => panic!("surface reacquire failed: {other:?}"),
                }
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => return,
            other => panic!("surface error: {other:?}"),
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("web frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("web ribbon pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(crate::CLEAR),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            self.renderer.draw(&mut pass);
        }
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
    }
}

#[wasm_bindgen]
pub async fn run() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;
    let canvas: web_sys::HtmlCanvasElement = document
        .get_element_by_id("canvas")
        .ok_or("no canvas")?
        .dyn_into()?;

    let width = canvas.width().max(1);
    let height = canvas.height().max(1);

    let instance = wgpu::util::new_instance_with_webgpu_detection(
        wgpu::InstanceDescriptor::new_without_display_handle(),
    )
    .await;
    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
        .map_err(|error| format!("no surface: {error}"))?;
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
            apply_limit_buckets: false,
        })
        .await
        .map_err(|error| format!("no adapter: {error}"))?;
    let info = adapter.get_info();
    log::info!(
        "adapter: {} ({:?}, {:?})",
        info.name,
        info.backend,
        info.device_type
    );

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .map_err(|error| format!("no device: {error}"))?;
    let config = surface
        .get_default_config(&adapter, width, height)
        .ok_or("surface has no formats")?;
    surface.configure(&device, &config);

    let (vertices, scene_radius) = build_scene();
    let renderer = Renderer::new(
        &device,
        &queue,
        config.format,
        &vertices,
        camera_matrix(width, height, 0.0, scene_radius),
    );

    let state = Rc::new(RefCell::new(WebState {
        surface,
        device,
        queue,
        renderer,
        config,
        scene_radius,
    }));

    let slot: Rc<RefCell<Option<FrameCallback>>> = Rc::new(RefCell::new(None));
    let slot_for_closure = slot.clone();
    let state_for_closure = state.clone();
    let mut clock = crate::FrameClock::default();
    *slot.borrow_mut() = Some(Closure::new(move |time: f64| {
        let seconds = clock.seconds(time);
        state_for_closure.borrow_mut().render(seconds);
        if let Some(closure) = slot_for_closure.borrow().as_ref() {
            request_animation_frame(closure);
        }
    }));
    if let Some(closure) = slot.borrow().as_ref() {
        request_animation_frame(closure);
    }
    std::mem::forget(slot);
    Ok(())
}

fn request_animation_frame(closure: &FrameCallback) {
    web_sys::window()
        .expect("no window")
        .request_animation_frame(closure.as_ref().unchecked_ref())
        .expect("requestAnimationFrame failed");
}
