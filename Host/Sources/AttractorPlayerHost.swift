import AppKit
import os
import QuartzCore

/// Drives one Rust player for one `CAMetalLayer`.
///
/// The player owns the simulation, the camera, the ribbon and the overlay; this
/// class only translates AppKit lifecycle events into the C ABI: create and
/// attach on start, tick and draw on a 60 Hz run-loop timer, resize when the
/// backing size changes, and destroy on stop. A health-monitor verdict and the
/// restart limiter stay inside the player, so the host has no restart state of
/// its own.
@MainActor
final class AttractorPlayerHost {
    /// Structured log, visible with:
    /// `log stream --level debug --predicate 'subsystem == "io.github.xekchansky.AttractorSaver"'`.
    private let logger = Logger(
        subsystem: "io.github.xekchansky.AttractorSaver",
        category: "player"
    )
    private let view: MetalSurfaceView
    /// The opaque player handle; nil between `stop` and the next `start`.
    private var player: OpaquePointer?
    private var timer: Timer?
    private var lastFrameTime: CFTimeInterval = 0
    private var width: UInt32 = 0
    private var height: UInt32 = 0
    private var overlayScale: Float = 0
    /// Set while a failure is being repeated, so the log is not spammed every
    /// frame. Cleared by the next successful call.
    private var reportingFailure = false

    init(view: MetalSurfaceView) {
        self.view = view
    }

    /// Create the player and start the frame loop.
    ///
    /// A second call while the player is running is a no-op and reports
    /// success. A failed creation or attach leaves no player behind and
    /// returns `false`, so the owner can drop this host and retry the next
    /// time the saver starts instead of latching a blank surface forever.
    @discardableResult
    func start() -> Bool {
        guard player == nil else { return true }

        let size = view.updateDrawableSize()
        var options = AttractorPlayerOptions(
            seed: 0,
            width: size.width,
            height: size.height,
            overlay_scale: Float(size.scale),
            use_noop_backend: 0
        )
        guard let handle = attractor_player_create(&options) else {
            logger.error("Cannot start the attractor player: no GPU or no catalog")
            return false
        }
        let layer = Unmanaged.passUnretained(view.metalLayer).toOpaque()
        let status = attractor_player_attach_layer(handle, layer)
        guard status == ATTRACTOR_PLAYER_OK.rawValue else {
            let message = errorMessage(handle)
            logger.error("Cannot attach the Metal layer: \(message, privacy: .public)")
            attractor_player_destroy(handle)
            return false
        }

        player = handle
        width = size.width
        height = size.height
        overlayScale = Float(size.scale)
        lastFrameTime = CACurrentMediaTime()

        // A run-loop timer matches the Swift `SimulationScene`: the screenshot
        // host never starts a display link, and a timer keeps the saver alive
        // in the preview pane as well as in fullscreen.
        let timer = Timer(timeInterval: 1.0 / 60.0, repeats: true) { [weak self] _ in
            MainActor.assumeIsolated {
                self?.tick()
            }
        }
        timer.tolerance = 1.0 / 240.0
        RunLoop.main.add(timer, forMode: .common)
        self.timer = timer
        return true
    }

    /// Stop the frame loop and destroy the player, releasing its GPU resources.
    func stop() {
        timer?.invalidate()
        timer = nil
        if let player {
            attractor_player_destroy(player)
        }
        player = nil
        overlayScale = 0
        reportingFailure = false
    }

    private func tick() {
        guard let player else { return }
        let now = CACurrentMediaTime()
        let interval = now - lastFrameTime
        lastFrameTime = now

        resizeIfNeeded(player)

        var retryAfter: Double = 0
        let status = attractor_player_tick(player, interval, &retryAfter)
        if status == ATTRACTOR_PLAYER_RESTART_DEFERRED.rawValue {
            // The player retries on its own once the limiter window elapses.
        } else if status == ATTRACTOR_PLAYER_OK.rawValue
            || status == ATTRACTOR_PLAYER_RESTARTED.rawValue
        {
            reportingFailure = false
        } else {
            reportFailure(player, what: "tick")
            return
        }

        let drawStatus = attractor_player_draw(player)
        if drawStatus != ATTRACTOR_PLAYER_OK.rawValue {
            reportFailure(player, what: "draw")
        } else {
            reportingFailure = false
        }
    }

    /// Tell the player about a new drawable size or backing scale.
    private func resizeIfNeeded(_ player: OpaquePointer) {
        let size = view.updateDrawableSize()
        let scale = Float(size.scale)
        guard size.width != width || size.height != height || scale != overlayScale else {
            return
        }
        let status = attractor_player_resize(player, size.width, size.height, scale)
        if status == ATTRACTOR_PLAYER_OK.rawValue {
            width = size.width
            height = size.height
            overlayScale = scale
        } else {
            reportFailure(player, what: "resize")
        }
    }

    private func reportFailure(_ player: OpaquePointer, what: String) {
        guard !reportingFailure else { return }
        reportingFailure = true
        let message = errorMessage(player)
        logger.error("""
            The attractor player failed to \(what, privacy: .public): \
            \(message, privacy: .public)
            """)
    }

    /// The last player error, or a placeholder when the player has no message.
    private func errorMessage(_ player: OpaquePointer?) -> String {
        guard let player, let message = attractor_player_last_error(player) else {
            return "no error message"
        }
        return String(cString: message)
    }
}
