package net.dastari.librarian.tv

import android.view.KeyEvent

/** What a remote key does in the app (RemoteKeysTest). Arrows and OK reach the page natively. */
sealed interface RemoteAction {
    /** Back: the page first (close, then go back), the app exits only from Home. Long press: server screen. */
    data object Back : RemoteAction

    /** Menu: the focused item's actions (`ContextMenu`). Long press: server screen. */
    data object Menu : RemoteAction

    /** A key the page handles, sent as a DOM key name (`KeyboardEvent.key`). */
    data class PageKey(val key: String, val repeats: Boolean = false) : RemoteAction
}

object RemoteKeys {
    fun actionFor(keyCode: Int): RemoteAction? = when (keyCode) {
        KeyEvent.KEYCODE_BACK, KeyEvent.KEYCODE_BUTTON_B -> RemoteAction.Back
        KeyEvent.KEYCODE_MENU, KeyEvent.KEYCODE_TV_CONTENTS_MENU -> RemoteAction.Menu
        KeyEvent.KEYCODE_INFO -> RemoteAction.PageKey("Info")
        KeyEvent.KEYCODE_GUIDE -> RemoteAction.PageKey("Guide")
        KeyEvent.KEYCODE_MEDIA_PLAY_PAUSE, KeyEvent.KEYCODE_HEADSETHOOK -> RemoteAction.PageKey("MediaPlayPause")
        KeyEvent.KEYCODE_MEDIA_PLAY -> RemoteAction.PageKey("MediaPlay")
        KeyEvent.KEYCODE_MEDIA_PAUSE -> RemoteAction.PageKey("MediaPause")
        KeyEvent.KEYCODE_MEDIA_STOP -> RemoteAction.PageKey("MediaStop")
        KeyEvent.KEYCODE_MEDIA_NEXT -> RemoteAction.PageKey("MediaTrackNext")
        KeyEvent.KEYCODE_MEDIA_PREVIOUS -> RemoteAction.PageKey("MediaTrackPrevious")
        KeyEvent.KEYCODE_MEDIA_REWIND, KeyEvent.KEYCODE_MEDIA_SKIP_BACKWARD, KeyEvent.KEYCODE_MEDIA_STEP_BACKWARD ->
            RemoteAction.PageKey("MediaRewind", repeats = true)
        KeyEvent.KEYCODE_MEDIA_FAST_FORWARD, KeyEvent.KEYCODE_MEDIA_SKIP_FORWARD, KeyEvent.KEYCODE_MEDIA_STEP_FORWARD ->
            RemoteAction.PageKey("MediaFastForward", repeats = true)
        KeyEvent.KEYCODE_CHANNEL_UP -> RemoteAction.PageKey("PageUp", repeats = true)
        KeyEvent.KEYCODE_CHANNEL_DOWN -> RemoteAction.PageKey("PageDown", repeats = true)
        else -> null
    }
}

/**
 * Tells a short press from a long one for keys whose long press does something else (Back and
 * Menu open the server screen). Remotes differ: some repeat the key while held, some send only
 * down and up, so both the repeats and the time held count (PressTrackerTest).
 */
class PressTracker(private val longPressMs: Long = 700) {
    enum class Release { SHORT, LONG, IGNORED }

    private var downAt = -1L
    private var fired = false

    /** A key-down; true when it completes a long press (act on it then, once). */
    fun down(eventTime: Long, repeatCount: Int, systemLongPress: Boolean = false): Boolean {
        if (repeatCount == 0 || downAt < 0) {
            downAt = eventTime
            fired = false
            return false
        }
        if (!fired && (systemLongPress || eventTime - downAt >= longPressMs)) {
            fired = true
            return true
        }
        return false
    }

    /** The key-up: SHORT to act now, LONG for a long press no repeat reported, IGNORED otherwise. */
    fun up(eventTime: Long, canceled: Boolean): Release {
        val start = downAt
        val alreadyFired = fired
        downAt = -1
        fired = false
        return when {
            start < 0 || canceled || alreadyFired -> Release.IGNORED
            eventTime - start >= longPressMs -> Release.LONG
            else -> Release.SHORT
        }
    }
}
