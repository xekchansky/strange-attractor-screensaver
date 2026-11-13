//! How the rendered trail is colored.

use serde::{Deserialize, Serialize};

/// Selects how the rendered trail is colored.
///
/// The mode is sampled randomly at startup (deterministically for a given
/// seed) and can be overridden by the demo's `--color-mode` flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TraceColorMode {
    /// Rainbow gradient along the trace (position of the point in the trail).
    Trail,
    /// Grayscale fade: older points darker, newer points brighter.
    Age,
    /// Color by the point's distance from the camera.
    Depth,
    /// Color by the point's coordinates: x→red, y→green, z→blue.
    Spatial,
}

impl TraceColorMode {
    /// Every mode, in declaration order.
    pub const ALL: [Self; 4] = [Self::Trail, Self::Age, Self::Depth, Self::Spatial];

    /// The stable lowercase name used in the JSON catalog and CLI flags.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Trail => "trail",
            Self::Age => "age",
            Self::Depth => "depth",
            Self::Spatial => "spatial",
        }
    }
}
