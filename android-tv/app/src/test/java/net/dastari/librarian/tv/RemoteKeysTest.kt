package net.dastari.librarian.tv

import android.view.KeyEvent
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class RemoteKeysTest {
    @Test
    fun backAndMenuAreTheShells() {
        assertEquals(RemoteAction.Back, RemoteKeys.actionFor(KeyEvent.KEYCODE_BACK))
        assertEquals(RemoteAction.Back, RemoteKeys.actionFor(KeyEvent.KEYCODE_BUTTON_B))
        assertEquals(RemoteAction.Menu, RemoteKeys.actionFor(KeyEvent.KEYCODE_MENU))
        assertEquals(RemoteAction.PageKey("Info"), RemoteKeys.actionFor(KeyEvent.KEYCODE_INFO))
        assertEquals(RemoteAction.PageKey("Guide"), RemoteKeys.actionFor(KeyEvent.KEYCODE_GUIDE))
    }

    @Test
    fun mediaKeysBecomeDomKeyNames() {
        val expected = mapOf(
            KeyEvent.KEYCODE_MEDIA_PLAY_PAUSE to "MediaPlayPause",
            KeyEvent.KEYCODE_HEADSETHOOK to "MediaPlayPause",
            KeyEvent.KEYCODE_MEDIA_PLAY to "MediaPlay",
            KeyEvent.KEYCODE_MEDIA_PAUSE to "MediaPause",
            KeyEvent.KEYCODE_MEDIA_STOP to "MediaStop",
            KeyEvent.KEYCODE_MEDIA_NEXT to "MediaTrackNext",
            KeyEvent.KEYCODE_MEDIA_PREVIOUS to "MediaTrackPrevious",
            KeyEvent.KEYCODE_MEDIA_REWIND to "MediaRewind",
            KeyEvent.KEYCODE_MEDIA_FAST_FORWARD to "MediaFastForward",
            KeyEvent.KEYCODE_MEDIA_SKIP_FORWARD to "MediaFastForward",
            KeyEvent.KEYCODE_CHANNEL_UP to "PageUp",
        )
        for ((code, key) in expected) {
            assertEquals("key code $code", key, (RemoteKeys.actionFor(code) as RemoteAction.PageKey).key)
        }
        assertEquals(true, (RemoteKeys.actionFor(KeyEvent.KEYCODE_MEDIA_REWIND) as RemoteAction.PageKey).repeats)
        assertEquals(false, (RemoteKeys.actionFor(KeyEvent.KEYCODE_MEDIA_PLAY_PAUSE) as RemoteAction.PageKey).repeats)
    }

    @Test
    fun arrowsAndOkReachThePageNatively() {
        for (code in listOf(KeyEvent.KEYCODE_DPAD_UP, KeyEvent.KEYCODE_DPAD_DOWN, KeyEvent.KEYCODE_DPAD_LEFT, KeyEvent.KEYCODE_DPAD_RIGHT, KeyEvent.KEYCODE_DPAD_CENTER, KeyEvent.KEYCODE_ENTER, KeyEvent.KEYCODE_ESCAPE)) {
            assertNull(RemoteKeys.actionFor(code))
        }
    }
}
