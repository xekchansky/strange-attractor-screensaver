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
        let n = trace.count
        guard n > 1 else { return SCNGeometry() }

        // Uniformly subsample very long trails to keep the polyline within a
        // vertex budget. The trail is a dense continuous curve, so a constant
        // stride preserves its shape; the color ramp is derived from the
        // original index proportion, keeping the gradient unchanged. Trails at
        // or below the budget are drawn untouched.
        let budget = 16_384
        let stride: Int = n <= budget ? 1 : (n + budget - 1) / budget
        var sampledIndices: [Int] = []
        sampledIndices.reserveCapacity((n - 1) / stride + 2)
        var index = 0
        while index < n {
            sampledIndices.append(index)
            index += stride
        }
        // Always keep the newest point (the trail head) as the final vertex.
        if let last = sampledIndices.last, last != n - 1 {
            sampledIndices.append(n - 1)
        }
        let sampledCount = sampledIndices.count

        var positions: [SCNVector3] = []
        positions.reserveCapacity(sampledCount)
        for index in sampledIndices {
            let state = trace[index]
            positions.append(
                SCNVector3(CGFloat(state.x), CGFloat(state.y), CGFloat(state.z))
            )
        }

        let count = Double(n)
        var colors: [Float] = []
        colors.reserveCapacity(sampledCount * 3)
        for index in sampledIndices {
            let hue = (Double(index) / count + colorOffset).truncatingRemainder(dividingBy: 1.0)
            let (r, g, b) = Self.hsbToRgb(hue: hue, saturation: 0.9, brightness: 1.0)
            colors.append(Float(r))
            colors.append(Float(g))
            colors.append(Float(b))
        }

        // Explicit paired line indices: SceneKit draws one segment per pair
        // (0,1), (1,2), ..., giving a single connected strip. An empty
        // index buffer (bytesPerIndex: 0) is not reliably handled by the
        // Metal renderer and can crash in drawIndexedPrimitives.
        let segmentCount = sampledCount - 1
        var indices: [UInt32] = []
        indices.reserveCapacity(segmentCount * 2)
        for i in 0..<segmentCount {
            indices.append(UInt32(i))
            indices.append(UInt32(i + 1))
        }
        let indexData = indices.withUnsafeBytes { Data($0) }

        let positionSource = SCNGeometrySource(vertices: positions)
        let colorSource = SCNGeometrySource(
            data: Data(bytes: colors, count: colors.count * MemoryLayout<Float>.size),
            semantic: .color,
            vectorCount: sampledCount,
            usesFloatComponents: true,
            componentsPerVector: 3,
            bytesPerComponent: MemoryLayout<Float>.size,
            dataOffset: 0,
            dataStride: MemoryLayout<Float>.size * 3
        )
        let element = SCNGeometryElement(
            data: indexData,
            primitiveType: .line,
            primitiveCount: segmentCount,
            bytesPerIndex: MemoryLayout<UInt32>.size
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
