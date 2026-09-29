import SceneKit
import AttractorCore

/// Converts the engine's trace into a SceneKit polyline with a color gradient
/// that shows the direction of motion along the trail.
///
/// This is a pure function of the trace (plus a time-varying hue offset), so it
/// can be reused directly by the screensaver host (M5) without UI coupling.
enum TraceRenderer {
    /// Build a polyline geometry from `trace`.
    ///
    /// - Parameters:
    ///   - trace: The ordered trace points, oldest first.
    ///   - colorOffset: A value in `[0, 1)` shifting the hue ramp. Advance it by
    ///     the `trailColorSpeed` every frame to animate the gradient.
    static func geometry(for trace: [State], colorOffset: Double) -> SCNGeometry {
        guard trace.count > 1 else { return SCNGeometry() }

        var positions: [SCNVector3] = []
        positions.reserveCapacity(trace.count)
        for state in trace {
            positions.append(
                SCNVector3(CGFloat(state.x), CGFloat(state.y), CGFloat(state.z))
            )
        }

        let count = Double(trace.count)
        var colors: [Float] = []
        colors.reserveCapacity(trace.count * 3)
        for i in 0..<trace.count {
            let hue = (Double(i) / count + colorOffset).truncatingRemainder(dividingBy: 1.0)
            let (r, g, b) = Self.hsbToRgb(hue: hue, saturation: 0.9, brightness: 1.0)
            colors.append(Float(r))
            colors.append(Float(g))
            colors.append(Float(b))
        }

        let positionSource = SCNGeometrySource(vertices: positions)
        let colorSource = SCNGeometrySource(
            data: Data(bytes: colors, count: colors.count * MemoryLayout<Float>.size),
            semantic: .color,
            vectorCount: trace.count,
            usesFloatComponents: true,
            componentsPerVector: 3,
            bytesPerComponent: MemoryLayout<Float>.size,
            dataOffset: 0,
            dataStride: MemoryLayout<Float>.size * 3
        )
        // No index data: `primitiveCount` line segments connect consecutive
        // vertex pairs in order, forming a single continuous strip.
        let element = SCNGeometryElement(
            data: Data(),
            primitiveType: .line,
            primitiveCount: trace.count - 1,
            bytesPerIndex: 0
        )
        let geometry = SCNGeometry(sources: [positionSource, colorSource], elements: [element])
        // Vertex colors feed the diffuse channel; disable lighting so the
        // gradient is shown as-is on the unlit polyline.
        let material = SCNMaterial()
        material.lightingModel = .constant
        geometry.firstMaterial = material
        return geometry
    }

    /// Convert an HSB color component (hue in `[0, 1)`) to linear RGB in `[0, 1]`.
    private static func hsbToRgb(
        hue: Double,
        saturation: Double,
        brightness: Double
    ) -> (Double, Double, Double) {
        let segment = Int(hue * 6) % 6
        let fraction = hue * 6 - Double(segment)
        let p = brightness * (1 - saturation)
        let q = brightness * (1 - fraction * saturation)
        let t = brightness * (1 - (1 - fraction) * saturation)
        switch segment {
        case 0: return (brightness, t, p)
        case 1: return (q, brightness, p)
        case 2: return (p, brightness, t)
        case 3: return (p, q, brightness)
        case 4: return (t, p, brightness)
        default: return (brightness, p, q)
        }
    }
}
