package com.xekchansky.strangeattractor

import android.view.Surface

/**
 * JNI entry points implemented in `rust/crates/attractor-ffi/src/android.rs`.
 *
 * The functions are instance methods of this object, so their JNI symbols are
 * `Java_com_xekchansky_strangeattractor_NativeBridge_<name>`.
 * `rust/crates/attractor-ffi/tests/android_bridge.rs` fails when the two
 * declarations drift apart.
 */
object NativeBridge {
    /** The call succeeded. */
    const val STATUS_OK = 0

    /** A degenerate run was replaced with a fresh selection. */
    const val STATUS_RESTARTED = 1

    /** A restart is deferred; the next frame retries. */
    const val STATUS_RESTART_DEFERRED = 2

    /** The run is unhealthy and was kept on screen. */
    const val STATUS_UNHEALTHY = 3

    /** The call failed; ask [lastError]. */
    const val STATUS_ERROR = 4

    init {
        System.loadLibrary("attractor_ffi")
    }

    /**
     * Create a player presenting into [surface]; zero means failure.
     *
     * A zero [seed] makes the native side invent one, so every wallpaper
     * activation draws a new attractor. An empty [attractor] draws a random
     * one; [overlayScale] is the display density.
     */
    external fun create(
        surface: Surface,
        width: Int,
        height: Int,
        seed: Long,
        attractor: String,
        overlayScale: Float,
    ): Long

    /** Advance by [elapsedNanos] and present one frame; returns a `STATUS_*`. */
    external fun render(handle: Long, elapsedNanos: Long): Int

    /** Retarget the player and its swap chain after a size change. */
    external fun resize(handle: Long, width: Int, height: Int): Int

    /** Release the player, its GPU context and its window reference. */
    external fun destroy(handle: Long)

    /** The last error of [handle]; zero reports a failed [create]. */
    external fun lastError(handle: Long): String?

    /** The catalog the settings activity lists, as a JSON array. */
    external fun catalog(): String?
}
