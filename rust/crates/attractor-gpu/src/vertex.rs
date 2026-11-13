//! The ribbon vertex format shared by the pipeline and the upload path.

use attractor_view::TrailGeometry;
use bytemuck::{Pod, Zeroable};

/// One ribbon vertex: a position and a linear RGB color.
///
/// The layout matches the `VertexInput` struct in [`crate::RIBBON_SHADER`].
/// `TrailGeometry` already rounds every coordinate through `f32`, so the cast
/// is lossless.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Vertex {
    /// World-space position.
    pub position: [f32; 3],
    /// Linear RGB color; the sRGB target encodes it on write.
    pub color: [f32; 3],
}

impl Vertex {
    /// Attribute descriptions for the pipeline.
    pub const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    /// The vertex buffer layout the pipeline expects.
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }

    /// Interleave a `TrailGeometry` into vertex buffer order.
    ///
    /// The geometry exposes positions and colors as parallel slices; the GPU
    /// wants them interleaved, one vertex per ribbon endpoint.
    pub fn from_geometry(geometry: &TrailGeometry) -> Vec<Self> {
        geometry
            .positions()
            .iter()
            .zip(geometry.colors())
            .map(|(position, color)| Self {
                position: [position.x as f32, position.y as f32, position.z as f32],
                color: [color.r, color.g, color.b],
            })
            .collect()
    }
}
