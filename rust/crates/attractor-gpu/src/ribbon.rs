//! The ribbon pipeline: one uniform (view-projection), one vertex buffer.

use attractor_view::{Mat4, TrailGeometry};
use bytemuck::{Pod, Zeroable};

use crate::context::GpuContext;
use crate::vertex::Vertex;

/// WGSL source of the ribbon pipeline.
///
/// The ribbon is one triangle strip with per-vertex linear colors. The shader
/// stays inside the WebGL2 subset (one uniform buffer, vertex buffers only) so
/// the same source compiles to GLSL for the browser fallback (ADR-1).
pub const RIBBON_SHADER: &str = r#"
struct Uniforms {
    view_projection: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = uniforms.view_projection * vec4<f32>(input.position, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct Uniforms {
    view_projection: [[f32; 4]; 4],
}

/// Draws a `TrailGeometry` as one camera-facing triangle strip.
///
/// The renderer reuses its vertex buffer across frames and only reallocates
/// when the trail grows past the current capacity, matching the buffer reuse
/// rationale of the Swift `TraceRenderer`.
pub struct RibbonRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    vertex_capacity: usize,
    vertex_count: u32,
}

impl RibbonRenderer {
    /// Build the pipeline for `format` with `sample_count` multisamples.
    pub fn new(context: &GpuContext, format: wgpu::TextureFormat, sample_count: u32) -> Self {
        let device = context.device();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ribbon shader"),
            source: wgpu::ShaderSource::Wgsl(RIBBON_SHADER.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ribbon camera layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ribbon camera uniform"),
            size: std::mem::size_of::<Uniforms>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ribbon camera bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ribbon pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ribbon pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(Vertex::layout())],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                // The ribbon winds either way as the camera orbits; the Swift
                // material is double-sided, so culling stays off.
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: sample_count,
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let vertex_buffer = Self::create_vertex_buffer(device, 0);
        Self {
            pipeline,
            uniform_buffer,
            bind_group,
            vertex_buffer,
            vertex_capacity: 0,
            vertex_count: 0,
        }
    }

    /// Number of vertices in the last uploaded geometry.
    pub const fn vertex_count(&self) -> u32 {
        self.vertex_count
    }

    /// Upload `geometry` into the vertex buffer.
    ///
    /// The buffer grows geometrically so a growing trail reallocates a handful
    /// of times instead of once per frame.
    pub fn upload(&mut self, context: &GpuContext, geometry: &TrailGeometry) {
        let vertices = Vertex::from_geometry(geometry);
        self.vertex_count = vertices.len() as u32;
        if vertices.is_empty() {
            return;
        }
        if vertices.len() > self.vertex_capacity {
            let capacity = vertices.len().next_power_of_two();
            self.vertex_buffer = Self::create_vertex_buffer(context.device(), capacity);
            self.vertex_capacity = capacity;
        }
        context
            .queue()
            .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
    }

    /// Upload the camera matrix used by the next draw.
    pub fn set_view_projection(&self, context: &GpuContext, view_projection: &Mat4) {
        let uniforms = Uniforms {
            view_projection: view_projection.as_columns(),
        };
        context
            .queue()
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    /// Record the draw call. The camera and vertices must already be uploaded.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.vertex_count < 3 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
    }

    fn create_vertex_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        // A zero-sized buffer is invalid, so an empty renderer keeps room for
        // one vertex until the first upload.
        let size = (capacity.max(1) * std::mem::size_of::<Vertex>()) as wgpu::BufferAddress;
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ribbon vertices"),
            size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }
}
