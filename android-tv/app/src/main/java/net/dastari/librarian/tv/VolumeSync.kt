package net.dastari.librarian.tv

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.database.ContentObserver
import android.media.AudioDeviceInfo
import android.media.AudioManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.provider.Settings
import android.util.Log
import org.json.JSONObject
import kotlin.math.roundToInt

/** Volume steps and the page's 0–1 level, with no drift either way (VolumeStepsTest). */
object VolumeSteps {
    fun level(step: Int, max: Int): Double = if (max <= 0) 0.0 else step.coerceIn(0, max).toDouble() / max

    fun step(level: Double, max: Int): Int =
        if (max <= 0 || level.isNaN()) 0 else (level.coerceIn(0.0, 1.0) * max).roundToInt().coerceIn(0, max)
}

/** What the page is told (`librarian-tv-volume` event, `LibrarianTV.volume()`). */
data class VolumeState(val step: Int, val max: Int, val muted: Boolean, val fixed: Boolean, val hdmiArc: Boolean) {
    fun toJson(): String = JSONObject()
        .put("level", VolumeSteps.level(step, max))
        .put("step", step)
        .put("max", max)
        .put("muted", muted)
        .put("fixed", fixed)
        .put("hdmiArc", hdmiArc)
        .toString()
}

/**
 * The TV's media volume (`STREAM_MUSIC`) for the page's volume control (docs/design.md Q137). The
 * remote's volume keys are never intercepted: Android handles them and shows its own volume UI,
 * and the change reaches the page through the `VOLUME_CHANGED_ACTION` broadcast and a settings
 * observer, whichever comes first.
 *
 * Volume counts as fixed when Android says so (`isVolumeFixed`) or when setting it had no effect
 * (some TVs hand volume to a sound system over HDMI-CEC). Then [setLevel] tries one
 * `adjustStreamVolume` raise or lower, which some TVs forward over CEC.
 */
class VolumeSync(context: Context, private val onChange: (VolumeState) -> Unit) {
    private val app = context.applicationContext
    private val audio = app.getSystemService(Context.AUDIO_SERVICE) as AudioManager
    private val main = Handler(Looper.getMainLooper())
    private var last: VolumeState? = null
    @Volatile
    private var ineffective = false
    private var listening = false

    private val receiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) = schedule()
    }
    private val observer = object : ContentObserver(main) {
        override fun onChange(selfChange: Boolean) = schedule()
    }
    private val publish = Runnable {
        val now = state()
        if (now != last) {
            last = now
            onChange(now)
        }
    }

    fun state(): VolumeState {
        val max = audio.getStreamMaxVolume(STREAM)
        val step = audio.getStreamVolume(STREAM)
        val muted = Build.VERSION.SDK_INT >= 23 && audio.isStreamMute(STREAM)
        return VolumeState(step, max, muted, audio.isVolumeFixed || ineffective, hdmiArc())
    }

    /** Set the level (0–1); the nearest whole step. No system UI: the page shows the change. */
    fun setLevel(level: Double) {
        val before = state()
        val target = VolumeSteps.step(level, before.max)
        if (target == before.step && !before.muted) return
        try {
            if (before.muted && target > 0) audio.adjustStreamVolume(STREAM, AudioManager.ADJUST_UNMUTE, 0)
            if (!before.fixed) audio.setStreamVolume(STREAM, target, 0)
            if (audio.getStreamVolume(STREAM) != target && target != before.step) {
                // Nothing moved: the TV hands volume to something else. Try a step that way, with
                // the system's UI (it may be the sound system's own), and report it as fixed.
                if (!ineffective) Log.i(TAG, "Setting the volume had no effect; treating it as fixed")
                ineffective = true
                val direction = if (target > before.step) AudioManager.ADJUST_RAISE else AudioManager.ADJUST_LOWER
                audio.adjustStreamVolume(STREAM, direction, AudioManager.FLAG_SHOW_UI)
            }
        } catch (error: SecurityException) {
            // Do Not Disturb or a policy refused it.
            Log.w(TAG, "Volume change refused", error)
        }
        schedule()
    }

    /** Mute or unmute; true when the TV's mute now matches. */
    fun setMuted(muted: Boolean): Boolean {
        if (Build.VERSION.SDK_INT < 23) return false
        return try {
            audio.adjustStreamVolume(STREAM, if (muted) AudioManager.ADJUST_MUTE else AudioManager.ADJUST_UNMUTE, 0)
            schedule()
            audio.isStreamMute(STREAM) == muted
        } catch (error: SecurityException) {
            Log.w(TAG, "Mute refused", error)
            false
        }
    }

    fun start() {
        if (listening) return
        listening = true
        val filter = IntentFilter().apply {
            addAction(ACTION_VOLUME_CHANGED)
            addAction(ACTION_MUTE_CHANGED)
        }
        if (Build.VERSION.SDK_INT >= 33) app.registerReceiver(receiver, filter, Context.RECEIVER_EXPORTED)
        else app.registerReceiver(receiver, filter)
        app.contentResolver.registerContentObserver(Settings.System.CONTENT_URI, true, observer)
        schedule()
    }

    fun stop() {
        if (!listening) return
        listening = false
        main.removeCallbacks(publish)
        try {
            app.unregisterReceiver(receiver)
        } catch (_: IllegalArgumentException) {
        }
        app.contentResolver.unregisterContentObserver(observer)
    }

    /** Changes arrive in bursts (a held key, both sources at once): publish once they settle. */
    private fun schedule() {
        main.removeCallbacks(publish)
        main.postDelayed(publish, 60)
    }

    private fun hdmiArc(): Boolean {
        if (Build.VERSION.SDK_INT < 23) return false
        return audio.getDevices(AudioManager.GET_DEVICES_OUTPUTS).any {
            it.type == AudioDeviceInfo.TYPE_HDMI_ARC || (Build.VERSION.SDK_INT >= 31 && it.type == AudioDeviceInfo.TYPE_HDMI_EARC)
        }
    }

    private companion object {
        const val TAG = "LibrarianVolume"
        const val STREAM = AudioManager.STREAM_MUSIC
        // Sent by AudioService for every stream volume and mute change (not in the public SDK).
        const val ACTION_VOLUME_CHANGED = "android.media.VOLUME_CHANGED_ACTION"
        const val ACTION_MUTE_CHANGED = "android.media.STREAM_MUTE_CHANGED_ACTION"
    }
}
