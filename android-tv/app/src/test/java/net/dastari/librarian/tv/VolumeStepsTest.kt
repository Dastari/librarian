package net.dastari.librarian.tv

import org.junit.Assert.assertEquals
import org.junit.Test

class VolumeStepsTest {
    @Test
    fun everyStepSurvivesTheRoundTrip() {
        for (max in listOf(1, 15, 25, 60, 100)) {
            for (step in 0..max) {
                assertEquals("max $max step $step", step, VolumeSteps.step(VolumeSteps.level(step, max), max))
            }
        }
    }

    @Test
    fun levelsGoToTheNearestStep() {
        assertEquals(6, VolumeSteps.step(0.37, 15))
        assertEquals(37, VolumeSteps.step(0.37, 100))
        assertEquals(15, VolumeSteps.step(1.4, 15))
        assertEquals(0, VolumeSteps.step(-0.2, 15))
        assertEquals(0, VolumeSteps.step(Double.NaN, 15))
        assertEquals(0.4, VolumeSteps.level(6, 15), 1e-9)
        assertEquals(1.0, VolumeSteps.level(20, 15), 1e-9)
        assertEquals(0.0, VolumeSteps.level(3, 0), 1e-9)
    }

    @Test
    fun aPageThatNudgesByFivePercentAlwaysMovesAStep() {
        // Glass's ↑/↓ change the level by 0.05; with 15 steps (6.7 % each) that still lands on the next one.
        var step = 6
        val up = VolumeSteps.step(VolumeSteps.level(step, 15) + 0.05, 15)
        assertEquals(7, up)
        step = up
        assertEquals(6, VolumeSteps.step(VolumeSteps.level(step, 15) - 0.05, 15))
    }
}

class RetryBackoffTest {
    @Test
    fun retriesSoonAtFirstThenEveryHalfMinute() {
        assertEquals(listOf(2_000L, 4_000L, 8_000L, 16_000L, 30_000L, 30_000L), (0..5).map(RetryBackoff::delayMs))
    }
}
