package com.xekchansky.strangeattractor

import android.service.wallpaper.WallpaperService
import android.util.Log
import android.view.SurfaceHolder
import java.util.concurrent.locks.LockSupport

/**
 * The live wallpaper.
 *
 * Every engine owns at most one rendering thread. The thread creates the
 * native player after the surface exists, draws at 60 Hz and destroys the
 * player before the surface goes away, so the GPU context never outlives the
 * window it presents into.
 */
class AttractorWallpaperService : WallpaperService() {
    override fun onCreateEngine(): Engine = WallpaperEngine()

    private inner class WallpaperEngine : Engine() {
        private var worker: RenderThread? = null

        @Volatile private var pendingWidth = 0

        @Volatile private var pendingHeight = 0

        override fun onSurfaceCreated(holder: SurfaceHolder) {
            super.onSurfaceCreated(holder)
            pendingWidth = holder.surfaceFrame.width()
            pendingHeight = holder.surfaceFrame.height()
            if (isVisible) start()
        }

        override fun onSurfaceChanged(
            holder: SurfaceHolder,
            format: Int,
            width: Int,
            height: Int,
        ) {
            super.onSurfaceChanged(holder, format, width, height)
            if (width > 0 && height > 0) {
                pendingWidth = width
                pendingHeight = height
            }
        }

        override fun onVisibilityChanged(visible: Boolean) {
            if (visible) start() else stop()
        }

        override fun onSurfaceDestroyed(holder: SurfaceHolder) {
            stop()
        }

        override fun onDestroy() {
            stop()
            super.onDestroy()
        }

        private fun start() {
            stop()
            val holder = surfaceHolder
            if (!holder.surface.isValid) return
            worker = RenderThread(holder).also { it.start() }
        }

        private fun stop() {
            worker?.stopRendering()
            worker = null
        }

        /**
         * The rendering thread of one surface.
         *
         * Size changes arrive on the main thread; the thread applies them
         * before the next frame instead of touching the native handle from two
         * threads.
         */
        private inner class RenderThread(
            private val holder: SurfaceHolder,
        ) : Thread("attractor-render") {
            @Volatile private var running = true

            private var handle = 0L

            override fun run() {
                val surface = holder.surface
                if (!surface.isValid) return
                var width = pendingWidth
                var height = pendingHeight
                if (width <= 0 || height <= 0) {
                    width = resources.displayMetrics.widthPixels
                    height = resources.displayMetrics.heightPixels
                }
                val density = resources.displayMetrics.density
                val attractor = Preferences.attractor(this@AttractorWallpaperService)
                handle =
                    NativeBridge.create(
                        surface,
                        width,
                        height,
                        0L,
                        attractor,
                        density,
                    )
                if (handle == 0L) {
                    Log.e(TAG, "cannot create the player: ${NativeBridge.lastError(0)}")
                    return
                }
                Log.i(TAG, "rendering $width x $height")
                var previous = System.nanoTime()
                while (running) {
                    val frameStart = System.nanoTime()
                    val elapsed = frameStart - previous
                    previous = frameStart
                    if (pendingWidth > 0 &&
                        pendingHeight > 0 &&
                        (pendingWidth != width || pendingHeight != height)
                    ) {
                        width = pendingWidth
                        height = pendingHeight
                        NativeBridge.resize(handle, width, height)
                    }
                    when (val status = NativeBridge.render(handle, elapsed)) {
                        NativeBridge.STATUS_RESTARTED ->
                            Log.i(TAG, "the run was degenerate; picked a new attractor")
                        NativeBridge.STATUS_UNHEALTHY,
                        NativeBridge.STATUS_ERROR,
                        ->
                            Log.w(TAG, "cannot draw: ${NativeBridge.lastError(handle)}")
                        else -> Unit
                    }
                    val remaining = FRAME_NANOS - (System.nanoTime() - frameStart)
                    if (remaining > 0) LockSupport.parkNanos(remaining)
                }
                NativeBridge.destroy(handle)
                handle = 0
            }

            fun stopRendering() {
                running = false
                join(STOP_TIMEOUT_MILLIS)
            }
        }
    }

    private companion object {
        const val TAG = "AttractorWallpaper"
        const val FRAME_NANOS = 16_666_667L
        const val STOP_TIMEOUT_MILLIS = 2_000L
    }
}
