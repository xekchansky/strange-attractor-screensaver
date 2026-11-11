import AppKit
import AttractorCore
import AttractorRender
import Metal
import SceneKit

/// Offscreen capture of the running simulation for documentation assets.
///
/// The demo still creates a real window (SceneKit needs a laid-out render
/// context), but every frame is rendered through an explicit `SCNRenderer`
/// rather than read back from the screen.
///
/// Two modes:
/// - ``Mode/screenshot``: one PNG after the scene has warmed up (the trail is
///   pre-filled by the caller and the camera has settled for a few frames).
/// - ``Mode/record``: a PNG of every `frameStride`-th rendered frame saved to
///   a directory, for later GIF/APNG assembly, then the app exits.
///
/// Frames are grabbed with `SCNView.snapshot()`, so the captured image matches
/// exactly what a viewer of the live animation would see.
@MainActor
final class CaptureController {
    enum Mode {
        /// Single still image after warm-up.
        case screenshot(outputPath: String)
        /// Sequence of still frames for later GIF/APNG assembly.
        case record(outputDirectory: String, seconds: Double, frameStride: Int)
    }

    /// Rendered frames to let the camera framing converge before a screenshot.
    private static let settleTicks = 90

    private let mode: Mode
    /// Snapshot resolution scale. 2x matches Retina displays: GitHub renders
    /// the 1200×900-sized layout at 2x on such screens, so a 1x capture looked
    /// blurry ("pixelated"). The offscreen renderer accepts any size.
    ///
    /// Recordings meant for a downscaled animation can use a smaller scale:
    /// the snapshot cost dominates the frame loop, and a cheaper snapshot lets
    /// the 60 Hz timer keep its cadence, which is what makes the captured
    /// motion smooth. Overridable per run with `--capture-scale`.
    private let captureScale: Int
    private var scene: SimulationScene?
    private var settleRemaining = 0
    private var renderedFrames = 0
    private var framesToCapture = 0
    private var frameStride = 1
    private var outputDirectory = ""
    /// Frames that could not be rendered or written. A capture that lost any
    /// frame exits non-zero so a calling script notices instead of assembling
    /// a recording with silent holes.
    private var failures = 0

    init(mode: Mode, captureScale: Int = 2) {
        self.mode = mode
        self.captureScale = captureScale
    }

    /// Install capture on a freshly started scene. Assumes the engine behind
    /// the scene was already fast-forwarded by the caller (full trail).
    func attach(to scene: SimulationScene) {
        self.scene = scene
        // Pin the simulated interval to the nominal 60 Hz tick: a snapshot
        // costs far more than one period, and the real intervals also drift as
        // the trail grows, so wall-clock stepping made captured motion uneven.
        // With a fixed interval every recorded frame advances the same amount
        // of simulated time, which is what makes the assembled animation smooth
        // and the capture reproducible.
        scene.fixedFrameInterval = 1.0 / 60.0
        scene.onFrameRendered = { [weak self] in self?.handleFrame() }
        switch mode {
        case .screenshot:
            settleRemaining = Self.settleTicks
        case .record(let directory, let seconds, let stride):
            outputDirectory = directory
            frameStride = max(1, stride)
            // The camera framing must converge before recording starts,
            // exactly as for a screenshot: otherwise the hero GIF shows the
            // attractor zooming into place for the first seconds.
            settleRemaining = Self.settleTicks
            // The scene ticks at 60 Hz, so one second of animation is 60
            // ticks. Capture every `frameStride`-th tick: the captured
            // sequence then plays back at the intended duration.
            framesToCapture = max(1, Int(ceil(seconds * 60.0 / Double(frameStride))))
        }
    }

    /// Called from the scene's per-frame hook (after the render pass).
    private func handleFrame() {
        guard scene != nil else { return }
        // Shared settle phase for both modes: let the camera framing converge
        // before the first captured frame so recordings start steady.
        if settleRemaining > 0 {
            settleRemaining -= 1
            return
        }
        switch mode {
        case .screenshot:
            saveSnapshot(to: screenshotPath())
            finish()
        case .record:
            renderedFrames += 1
            guard renderedFrames % frameStride == 0 else { return }
            let index = renderedFrames / frameStride
            saveSnapshot(to: outputDirectory + "/" + Self.frameFilename(index: index))
            if index >= framesToCapture {
                finish()
            }
        }
    }

    /// Record a failed frame so ``finish()`` can report it.
    private func registerFailure() {
        failures += 1
    }

    private func screenshotPath() -> String {
        guard case .screenshot(let path) = mode else { return "" }
        return path
    }

    private func saveSnapshot(to path: String) {
        // `SCNView.snapshot()` crashes in a background GUI session (no
        // WindowServer connection); render through an offscreen SCNRenderer
        // backed by an explicit Metal device instead.
        guard let scene = scene?.view.scene,
              let device = MTLCreateSystemDefaultDevice() else {
            NSLog("CaptureController: failed to create offscreen renderer")
            registerFailure()
            return
        }
        let renderer = SCNRenderer(device: device, options: nil)
        renderer.scene = scene
        renderer.pointOfView = scene.rootNode.childNode(withName: "camera", recursively: false)
            ?? scene.rootNode.childNodes.first(where: { $0.camera != nil })
        let image = renderer.snapshot(
            atTime: CACurrentMediaTime(),
            with: CGSize(
                width: 1200 * captureScale,
                height: 900 * captureScale
            ),
            antialiasingMode: .multisampling4X
        )
        guard let tiff = image.tiffRepresentation,
              let rep = NSBitmapImageRep(data: tiff),
              let png = rep.representation(using: .png, properties: [:]) else {
            NSLog("CaptureController: failed to encode PNG for \(path)")
            registerFailure()
            return
        }
        do {
            let url = URL(fileURLWithPath: path)
            try FileManager.default.createDirectory(
                at: url.deletingLastPathComponent(), withIntermediateDirectories: true
            )
            try png.write(to: url)
        } catch {
            NSLog("CaptureController: failed to write \(path): \(error)")
            registerFailure()
        }
    }

    /// Stop the loop and quit the application once the capture is complete.
    /// Uses `exit` directly: in capture runs the app quits by itself, and
    /// `NSApp.terminate` depends on app-lifecycle plumbing that may not be
    /// delivered in a background GUI context. Losing any frame makes the exit
    /// status non-zero so a calling script can retry instead of publishing a
    /// recording with silent holes.
    private func finish() {
        scene?.onFrameRendered = nil
        scene?.stop()
        if failures > 0 {
            NSLog("CaptureController: capture finished with \(failures) failed frame(s)")
            exit(1)
        }
        NSLog("CaptureController: capture complete")
        exit(0)
    }

    private static func frameFilename(index: Int) -> String {
        String(format: "frame_%05d.png", index)
    }
}
