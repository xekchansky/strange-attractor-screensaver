package com.xekchansky.strangeattractor

import android.content.Context

/**
 * The attractor choice, stored where Android keeps per-app settings.
 *
 * The key and the "empty means random" rule match the desktop shells'
 * `settings.json`, so the same vocabulary describes every platform.
 */
object Preferences {
    private const val FILE_NAME = "settings"
    private const val KEY_ATTRACTOR = "attractor"

    /** The pinned attractor id, or an empty string for "random each time". */
    fun attractor(context: Context): String =
        context
            .getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .getString(KEY_ATTRACTOR, "")
            ?: ""

    /** Pin an attractor id; a blank id goes back to random. */
    fun setAttractor(context: Context, id: String) {
        val editor =
            context
                .getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
                .edit()
        if (id.isBlank()) {
            editor.remove(KEY_ATTRACTOR)
        } else {
            editor.putString(KEY_ATTRACTOR, id)
        }
        editor.apply()
    }
}
