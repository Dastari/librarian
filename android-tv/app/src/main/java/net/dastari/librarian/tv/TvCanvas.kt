package net.dastari.librarian.tv

import android.content.Context
import kotlin.math.roundToInt

/**
 * The 10-foot canvas (docs/design.md Q137). A 1080p TV reports density 320 (devicePixelRatio 2),
 * so the page's `width=device-width` gives it a 960 × 540 CSS viewport: a phone layout blown up.
 * The shell rewrites the page's viewport meta to `width = 1920 × 100 / display size`: 1920 CSS px
 * at 100 %, 1600 at 120 %, 2400 at 80 %, scaled to fit the screen. The canvas is fixed rather than
 * taken from the panel's pixels, so a TV whose app window really is 3840 px wide (density 640) gets
 * the same 10-foot layout instead of a quarter-size one; on the 1080p TCL G07 both are 1920. Chromium applies a changed viewport
 * at once, so a new display size needs no reload. (Building the WebView on a context with a lower
 * densityDpi was tried on the TCL G07 and does not work: the WebView ignores the override, or
 * renders blank on a bare configuration context.) Pure, and unit-tested (TvCanvasTest).
 */
object TvCanvas {
    const val CANVAS_WIDTH = 1920
    val DISPLAY_SIZES = listOf(80, 90, 100, 110, 120)

    fun normalizeSize(percent: Int): Int = if (percent in DISPLAY_SIZES) percent else 100

    /** The page's CSS width at a display size. */
    fun viewportWidth(percent: Int): Int = (CANVAS_WIDTH * 100.0 / normalizeSize(percent)).roundToInt()

    /** The script that sets the viewport; it only writes when the content differs. */
    fun viewportScript(width: Int): String =
        "(function(){var c='width=$width, viewport-fit=cover';" +
            "var m=document.querySelector('meta[name=\"viewport\"]');" +
            "if(!m){m=document.createElement('meta');m.setAttribute('name','viewport');" +
            "(document.head||document.documentElement).appendChild(m);}" +
            "if(m.getAttribute('content')!==c)m.setAttribute('content',c);})()"
}

/** This TV's display size, kept by the shell; the page reads and sets it through the bridge. */
object DisplayPrefs {
    private const val PREFS = "display"
    private const val KEY_SIZE = "size"

    fun size(context: Context): Int =
        TvCanvas.normalizeSize(context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getInt(KEY_SIZE, 100))

    fun setSize(context: Context, percent: Int) {
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit().putInt(KEY_SIZE, TvCanvas.normalizeSize(percent)).apply()
    }
}
