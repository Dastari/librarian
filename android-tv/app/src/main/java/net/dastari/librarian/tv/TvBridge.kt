package net.dastari.librarian.tv

import android.webkit.JavascriptInterface
import java.lang.ref.WeakReference

/**
 * `window.LibrarianTV` in the page (web/src/lib/tv-app.ts). Calls arrive on a WebView thread and
 * are ignored unless the page is the configured server's: the WebView never navigates elsewhere,
 * and this is the second guard.
 */
class TvBridge(activity: WebActivity) {
    private val activity = WeakReference(activity)

    @JavascriptInterface
    fun version(): String = BuildConfig.VERSION_NAME

    @JavascriptInterface
    fun serverUrl(): String = activity.get()?.serverUrl ?: ""

    /** "Display size" in percent (80–120); the page shows it in Settings › General. */
    @JavascriptInterface
    fun displaySize(): Int = activity.get()?.let(DisplayPrefs::size) ?: 100

    /** Store a new display size; the viewport changes at once, without a reload. */
    @JavascriptInterface
    fun setDisplaySize(percent: Int) {
        val host = activity.get() ?: return
        if (host.isServerPage()) host.runOnUiThread { host.changeDisplaySize(percent) }
    }

    /** The TV's media volume as JSON: `{level, step, max, muted, fixed, hdmiArc}`. */
    @JavascriptInterface
    fun volume(): String = activity.get()?.volumeState()?.toJson() ?: "null"

    /** Set the TV's media volume, 0 to 1 (rounded to a whole step). */
    @JavascriptInterface
    fun setVolume(level: Double) {
        val host = activity.get() ?: return
        if (host.isServerPage()) host.setVolumeLevel(level)
    }

    /** Mute or unmute the TV; false when it could not (the page then mutes the player instead). */
    @JavascriptInterface
    fun setMuted(muted: Boolean): Boolean {
        val host = activity.get() ?: return false
        return host.isServerPage() && host.setVolumeMuted(muted)
    }

    @JavascriptInterface
    fun setKeepScreenOn(on: Boolean) {
        val host = activity.get() ?: return
        if (host.isServerPage()) host.runOnUiThread { host.keepScreenOn(on) }
    }

    @JavascriptInterface
    fun openServerSettings() {
        val host = activity.get() ?: return
        if (host.isServerPage()) host.runOnUiThread { host.openServerSettings() }
    }
}
