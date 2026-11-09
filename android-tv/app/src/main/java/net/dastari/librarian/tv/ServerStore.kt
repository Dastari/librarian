package net.dastari.librarian.tv

import android.content.Context

/** The remembered server. Cookies (the sign-in) live in the WebView's own store, per origin. */
object ServerStore {
    private const val PREFS = "server"
    private const val KEY_URL = "url"

    fun get(context: Context): String? =
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getString(KEY_URL, null)?.takeIf { it.isNotBlank() }

    fun set(context: Context, url: String) {
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit().putString(KEY_URL, url).apply()
    }
}
