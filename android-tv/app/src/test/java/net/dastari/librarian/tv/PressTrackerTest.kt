package net.dastari.librarian.tv

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class PressTrackerTest {
    @Test
    fun aQuickPressIsShort() {
        val press = PressTracker(700)
        assertFalse(press.down(1_000, 0))
        assertEquals(PressTracker.Release.SHORT, press.up(1_120, canceled = false))
    }

    @Test
    fun aHeldKeyThatRepeatsFiresOnceWhileHeld() {
        val press = PressTracker(700)
        assertFalse(press.down(1_000, 0))
        assertFalse(press.down(1_400, 1))
        assertTrue(press.down(1_750, 2))
        assertFalse(press.down(1_800, 3))
        assertEquals(PressTracker.Release.IGNORED, press.up(2_000, canceled = false))
    }

    @Test
    fun theSystemsLongPressFlagCounts() {
        val press = PressTracker(700)
        press.down(1_000, 0)
        assertTrue(press.down(1_300, 1, systemLongPress = true))
    }

    @Test
    fun aRemoteThatDoesNotRepeatIsMeasuredOnRelease() {
        val press = PressTracker(700)
        press.down(1_000, 0)
        assertEquals(PressTracker.Release.LONG, press.up(1_900, canceled = false))
    }

    @Test
    fun aStrayOrCanceledReleaseDoesNothing() {
        val press = PressTracker(700)
        assertEquals(PressTracker.Release.IGNORED, press.up(1_000, canceled = false))
        press.down(1_000, 0)
        assertEquals(PressTracker.Release.IGNORED, press.up(1_100, canceled = true))
    }
}
