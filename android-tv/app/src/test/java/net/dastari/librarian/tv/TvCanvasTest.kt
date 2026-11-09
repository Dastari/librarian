package net.dastari.librarian.tv

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class TvCanvasTest {
    @Test
    fun theCanvasIs1920CssPxScaledByTheDisplaySize() {
        // The TCL G07: 1920 × 1080 at density 320. Measured on it: 1920 × 1080 at 100 %, 1600 × 900 at 120 %.
        assertEquals(1920, TvCanvas.viewportWidth(100))
        assertEquals(1600, TvCanvas.viewportWidth(120))
        assertEquals(1745, TvCanvas.viewportWidth(110))
        assertEquals(2400, TvCanvas.viewportWidth(80))
    }

    @Test
    fun unknownSizesFallBack() {
        assertEquals(1920, TvCanvas.viewportWidth(300))
        assertEquals(100, TvCanvas.normalizeSize(95))
    }

    @Test
    fun theScriptWritesTheWidthOnlyWhenItChanged() {
        val script = TvCanvas.viewportScript(1600)
        assertTrue(script.contains("width=1600, viewport-fit=cover"))
        assertTrue(script.contains("!==c"))
        assertTrue(script.startsWith("(function(){") && script.endsWith("})()"))
    }
}
