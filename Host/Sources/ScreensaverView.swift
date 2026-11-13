import AppKit
import os
import ScreenSaver

/// The classic `.saver` view.
///
/// The system instantiates this class (via `NSPrincipalClass`) and drives it
/// through `startAnimation`/`stopAnimation`/`animateOneFrame`. The view itself
/// owns no simulation: it hosts a `CAMetalLayer` and one Rust player, which
/// integrates, orbits, draws the ribbon and paints the overlay. Lifecycle
/// events map one-to-one onto the player's create/destroy calls.
@MainActor
final class ScreensaverView: ScreenSaver.ScreenSaverView {
    /// Structured log, visible with:
    /// `log stream --level debug --predicate 'subsystem == "io.github.xekchansky.AttractorSaver"'`.
    private let logger = Logger(
        subsystem: "io.github.xekchansky.AttractorSaver",
        category: "lifecycle"
    )
    /// The layer-backed view the player draws into.
    private var surfaceView: MetalSurfaceView?
    /// The running player; nil between `stopAnimation` and the next start.
    private var host: AttractorPlayerHost?

    override func startAnimation() {
        super.startAnimation()
        // Reuse a still-running player (re-entrant start); otherwise rebuild.
        guard host == nil else { return }
        tearDown()

        let surface = MetalSurfaceView(frame: bounds)
        surface.autoresizingMask = [.width, .height]
        addSubview(surface)
        let host = AttractorPlayerHost(view: surface)
        // A host without a player is dropped: caching it would make every later
        // `startAnimation` a no-op and leave a blank surface for the session.
        guard host.start() else {
            surface.removeFromSuperview()
            return
        }
        self.surfaceView = surface
        self.host = host
    }

    override func stopAnimation() {
        tearDown()
        super.stopAnimation()
    }

    override func viewDidMoveToSuperview() {
        super.viewDidMoveToSuperview()
        // The preview host never calls `stopAnimation()` and keeps every
        // `ScreensaverView` alive across preview launches. The only signal that
        // a preview ended is the view being detached, so release the player and
        // its GPU resources then; otherwise every launch would leak one player.
        if superview == nil {
            tearDown()
        }
    }

    /// The per-frame loop is driven by the host's timer, so the framework's
    /// callback stays empty.
    override func animateOneFrame() {}

    override func draw(_ dirtyRect: NSRect) {
        // The Metal surface fills `bounds`; nothing further to draw here.
    }

    private func tearDown() {
        host?.stop()
        host = nil
        surfaceView?.removeFromSuperview()
        surfaceView = nil
    }
}
