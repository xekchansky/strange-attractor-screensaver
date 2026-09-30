import Foundation

/// Selects how the rendered trail is colored.
///
/// The mode is sampled randomly at startup by ``Randomizer`` (deterministic for
/// a given seed) and can be overridden via the demo's `--color-mode` flag.
/// Whatever the mode, the animated ``SimulationSettings/trailColorSpeed``
/// gradient offset is applied to the ramp.
public enum TraceColorMode: String, CaseIterable, Codable, Sendable {
    /// Rainbow gradient along the trace (position of the point in the trail).
    case trail
    /// Grayscale fade: older points darker, newer points brighter.
    case age
    /// Color by the point's distance from the camera (depth).
    case depth
    /// Color by the point's coordinates: x→red, y→green, z→blue.
    case spatial
}
