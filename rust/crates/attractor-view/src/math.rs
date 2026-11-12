//! Column-major view matrices for the GPU-free camera layer.

use crate::vector::Vector3;

/// A 4×4 column-major matrix, the layout WGSL expects for `mat4x4<f32>`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    columns: [[f32; 4]; 4],
}

impl Mat4 {
    /// A view matrix looking from `eye` at `center`, with `up` as the world up
    /// axis (right-handed, camera looking down its local `-z`).
    ///
    /// This is the transform SceneKit derives from `SCNNode.look(at:)`, which
    /// is how the Swift camera orients itself.
    pub fn look_at(eye: Vector3, center: Vector3, up: Vector3) -> Self {
        let forward = center.subtract(eye).normalized();
        let side = forward.cross(up).normalized();
        let up = side.cross(forward);
        Self {
            columns: [
                [side.x as f32, up.x as f32, -forward.x as f32, 0.0],
                [side.y as f32, up.y as f32, -forward.y as f32, 0.0],
                [side.z as f32, up.z as f32, -forward.z as f32, 0.0],
                [
                    -side.dot(eye) as f32,
                    -up.dot(eye) as f32,
                    forward.dot(eye) as f32,
                    1.0,
                ],
            ],
        }
    }

    /// One column of the matrix.
    ///
    /// # Panics
    ///
    /// Panics when `index` is greater than three.
    pub fn column(&self, index: usize) -> [f32; 4] {
        self.columns[index]
    }

    /// Transform a world-space point into view space.
    pub fn transform_point(&self, point: Vector3) -> Vector3 {
        let [x, y, z] = [point.x as f32, point.y as f32, point.z as f32];
        // One matrix row spans the same component of every column.
        let row = |index: usize| {
            self.columns[0][index] * x
                + self.columns[1][index] * y
                + self.columns[2][index] * z
                + self.columns[3][index]
        };
        Vector3::new(f64::from(row(0)), f64::from(row(1)), f64::from(row(2)))
    }
}
