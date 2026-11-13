package com.xekchansky.strangeattractor

import android.app.Activity
import android.app.WallpaperManager
import android.content.ComponentName
import android.content.Intent
import android.os.Bundle
import android.view.ViewGroup
import android.widget.ArrayAdapter
import android.widget.Button
import android.widget.LinearLayout
import android.widget.Spinner
import android.widget.TextView
import android.widget.Toast
import org.json.JSONArray

/**
 * The attractor picker.
 *
 * The list comes from the native catalog, so the picker can only offer what
 * the player can actually run. The system wallpaper picker opens this activity
 * through `android:settingsActivity` in `res/xml/wallpaper.xml`; the launcher
 * icon opens the same screen, with an extra button that sets the wallpaper.
 */
class SettingsActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val ids = mutableListOf("")
        val labels = mutableListOf(getString(R.string.attractor_random))
        NativeBridge.catalog()?.let { json ->
            runCatching { JSONArray(json) }.getOrNull()?.let { array ->
                for (index in 0 until array.length()) {
                    val entry = array.getJSONObject(index)
                    ids.add(entry.getString("id"))
                    labels.add(entry.getString("name"))
                }
            }
        }

        val spinner =
            Spinner(this).apply {
                adapter =
                    ArrayAdapter(
                        this@SettingsActivity,
                        android.R.layout.simple_spinner_item,
                        labels,
                    ).apply { setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item) }
                val current = ids.indexOf(Preferences.attractor(this@SettingsActivity))
                if (current >= 0) setSelection(current)
            }

        val margin = (24 * resources.displayMetrics.density).toInt()
        val layout =
            LinearLayout(this).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(margin, margin, margin, margin)
                addView(
                    TextView(this@SettingsActivity).apply {
                        text = getString(R.string.settings_intro)
                    },
                )
                addView(
                    spinner,
                    LinearLayout.LayoutParams(
                        ViewGroup.LayoutParams.MATCH_PARENT,
                        ViewGroup.LayoutParams.WRAP_CONTENT,
                    ),
                )
                addView(
                    Button(this@SettingsActivity).apply {
                        text = getString(R.string.save)
                        setOnClickListener {
                            save(ids[spinner.selectedItemPosition])
                            Toast.makeText(
                                this@SettingsActivity,
                                R.string.saved,
                                Toast.LENGTH_SHORT,
                            ).show()
                        }
                    },
                )
                addView(
                    Button(this@SettingsActivity).apply {
                        text = getString(R.string.set_wallpaper)
                        setOnClickListener {
                            save(ids[spinner.selectedItemPosition])
                            setWallpaper()
                        }
                    },
                )
            }
        setContentView(layout)
    }

    private fun save(id: String) {
        Preferences.setAttractor(this, id)
    }

    private fun setWallpaper() {
        val intent =
            Intent(WallpaperManager.ACTION_CHANGE_LIVE_WALLPAPER)
                .putExtra(
                    WallpaperManager.EXTRA_LIVE_WALLPAPER_COMPONENT,
                    ComponentName(this, AttractorWallpaperService::class.java),
                )
        startActivity(intent)
    }
}
