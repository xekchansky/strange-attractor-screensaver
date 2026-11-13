import AppKit
import Metal
import QuartzCore

/// An `NSView` whose backing layer is a `CAMetalLayer`.
///
/// The Rust player receives this layer and presents its frames into it. The
/// view keeps the layer alive for as long as it is in the hierarchy; the
/// player is torn down before the view is released.
final class MetalSurfaceView: NSView {
    /// The layer handed to the player as the presentation target.
    let metalLayer = CAMetalLayer()

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        configureLayer()
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        configureLayer()
    }

    override var isOpaque: Bool { true }

    private func configureLayer() {
        wantsLayer = true
        layerContentsRedrawPolicy = .duringViewResize
        layer = metalLayer
        metalLayer.device = MTLCreateSystemDefaultDevice()
        metalLayer.pixelFormat = .bgra8Unorm
        metalLayer.framebufferOnly = true
        metalLayer.isOpaque = true
        updateDrawableSize()
    }

    override func viewDidChangeBackingProperties() {
        super.viewDidChangeBackingProperties()
        updateDrawableSize()
    }

    override func setFrameSize(_ newSize: NSSize) {
        super.setFrameSize(newSize)
        updateDrawableSize()
    }

    /// Recompute the drawable size from the bounds and the backing scale.
    ///
    /// Returns the pixel size the player should draw at and the scale used for
    /// the overlay. The values are also applied to the layer, so the swap chain
    /// and the drawable agree even before the player is attached.
    @discardableResult
    func updateDrawableSize() -> (width: UInt32, height: UInt32, scale: CGFloat) {
        let scale = window?.backingScaleFactor ?? NSScreen.main?.backingScaleFactor ?? 1
        let width = UInt32(max((bounds.width * scale).rounded(), 1))
        let height = UInt32(max((bounds.height * scale).rounded(), 1))
        metalLayer.contentsScale = scale
        metalLayer.drawableSize = CGSize(width: Int(width), height: Int(height))
        return (width, height, scale)
    }
}
