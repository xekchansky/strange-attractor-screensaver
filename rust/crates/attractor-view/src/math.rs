//! Column-major view matrices for the GPU-free camera layer.

use crate::vector::Vector3;

/// A 4×4 column-major matrix, the layout WGSL expects for `mat4x4<f32>`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    columns: [[f32; 4]; 4],
}

impl Mat4 {
    /// The identity transform.
    pub const fn identity() -> Self {
        Self {
            columns: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// A perspective projection for a right-handed camera looking down its
    /// local `-z`, with clip-space depth in `[0, 1]` (the wgpu convention).
    ///
    /// `field_of_view_y` is the full vertical field of view in radians. The
    /// Swift host uses SceneKit's default 60-degree field of view, measured
    /// along the larger view dimension (horizontal for a landscape window);
    /// the caller converts it to a vertical angle before calling this.
    ///
    /// Degenerate parameters (a non-positive or non-finite field of view,
    /// aspect ratio, near or far plane, or `near >= far`) would produce a
    /// non-invertible matrix or NaNs, so they fall back to the identity.
    pub fn perspective(field_of_view_y: f64, aspect: f64, near: f64, far: f64) -> Self {
        let tan_half = (field_of_view_y / 2.0).tan();
        if !(field_of_view_y.is_finite()
            && aspect.is_finite()
            && near.is_finite()
            && far.is_finite()
            && field_of_view_y > 0.0
            && field_of_view_y < std::f64::consts::PI
            && aspect > 0.0
            && near > 0.0
            && far > near
            && tan_half > 0.0)
        {
            return Self::identity();
        }
        let focal = 1.0 / tan_half;
        Self {
            columns: [
                [(focal / aspect) as f32, 0.0, 0.0, 0.0],
                [0.0, focal as f32, 0.0, 0.0],
                [0.0, 0.0, (far / (near - far)) as f32, -1.0],
                [0.0, 0.0, ((near * far) / (near - far)) as f32, 0.0],
            ],
        }
    }

    /// The matrix product `self * other`: `other` is applied to a point first.
    pub fn multiply(&self, other: &Self) -> Self {
        let mut columns = [[0.0f32; 4]; 4];
        for (index, column) in columns.iter_mut().enumerate() {
            for (row, value) in column.iter_mut().enumerate() {
                *value = (0..4)
                    .map(|k| self.columns[k][row] * other.columns[index][k])
                    .sum();
            }
        }
        Self { columns }
    }

    /// All four columns in memory order, as a vertex-stage uniform expects.
    pub const fn as_columns(&self) -> [[f32; 4]; 4] {
        self.columns
    }

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

    /// Transform a point into homogeneous clip space, without the perspective
    /// divide.
    pub fn project(&self, point: Vector3) -> [f32; 4] {
        let vector = [point.x as f32, point.y as f32, point.z as f32, 1.0];
        let mut clip = [0.0f32; 4];
        for (row, value) in clip.iter_mut().enumerate() {
            *value = (0..4).map(|k| self.columns[k][row] * vector[k]).sum();
        }
        clip
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
