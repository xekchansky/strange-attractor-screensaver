//! A three-dimensional vector with the precision of Swift's `CGFloat`.

/// A point or direction in world space.
///
/// The components are `f64`, like SceneKit's `SCNVector3` on 64-bit Apple
/// platforms; the camera and the ribbon builder therefore perform all of their
/// arithmetic at the same precision as the Swift oracle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector3 {
    /// Component along the first axis.
    pub x: f64,
    /// Component along the second axis.
    pub y: f64,
    /// Component along the third axis.
    pub z: f64,
}

impl Vector3 {
    /// The origin, `(0, 0, 0)`.
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    /// The world up axis, `(0, 1, 0)`.
    pub const UP: Self = Self::new(0.0, 1.0, 0.0);
    /// A vector shorter than this is treated as degenerate by [`Self::normalized`].
    const DEGENERATE_LENGTH: f64 = 1e-9;

    /// Build a vector from its three components.
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Euclidean length.
    pub fn length(self) -> f64 {
        self.dot(self).sqrt()
    }

    /// Euclidean distance to `other`.
    pub fn distance(self, other: Self) -> f64 {
        self.subtract(other).length()
    }

    /// Dot product.
    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Right-handed cross product.
    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    /// Component-wise difference.
    pub fn subtract(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }

    /// Multiply every component by `factor`.
    pub fn scale(self, factor: f64) -> Self {
        Self::new(self.x * factor, self.y * factor, self.z * factor)
    }

    /// Component-wise linear interpolation, `t = 0` giving `self`.
    pub fn lerp(self, other: Self, t: f64) -> Self {
        self + other.subtract(self).scale(t)
    }

    /// A unit vector in the same direction.
    ///
    /// A vector shorter than `1e-9` has no usable direction; the world up axis
    /// is returned instead, mirroring the Swift renderer so a degenerate trail
    /// still yields a finite ribbon.
    pub fn normalized(self) -> Self {
        let length = self.length();
        if length > Self::DEGENERATE_LENGTH {
            self.scale(1.0 / length)
        } else {
            Self::UP
        }
    }

    /// Round every component through `f32`.
    ///
    /// SceneKit stores node positions and vertex data as 32-bit floats even
    /// though its API is `CGFloat`-typed. Rounding here keeps the Rust camera
    /// and ribbon bit-identical to the Swift oracle's readable values and to
    /// the buffer the GPU will receive.
    pub fn rounded_to_f32(self) -> Self {
        Self::new(
            f64::from(self.x as f32),
            f64::from(self.y as f32),
            f64::from(self.z as f32),
        )
    }
}

impl std::ops::Add for Vector3 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}
