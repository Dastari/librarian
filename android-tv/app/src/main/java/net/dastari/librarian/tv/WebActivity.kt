package net.dastari.librarian.tv

import android.annotation.SuppressLint
import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.ComponentCallbacks2
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.Color
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.KeyEvent
import android.view.View
import android.view.ViewGroup
import android.view.WindowManager
import android.webkit.CookieManager
import android.webkit.PermissionRequest
import android.webkit.RenderProcessGoneDetail
import android.webkit.WebChromeClient
import android.webkit.WebResourceError
import android.webkit.WebResourceRequest
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.Button
import android.widget.FrameLayout
import android.widget.TextView
import org.json.JSONObject

/**
 * The whole app: one full-screen WebView showing the server's web app (docs/design.md Q137).
 *
 * - The page is laid out on a 1920 CSS px canvas by rewriting its viewport (`TvCanvas`).
 * - Back and Menu go to the page (web/src/lib/tv-app.ts); holding either opens the server screen.
 *   Media, Info and Guide buttons reach the page as standard key events.
 * - The page's volume control is the TV's own (`VolumeSync`).
 * - A crashed or reclaimed renderer is replaced and the page reloaded where it was. An unreachable
 *   server is retried with backoff behind a "Reconnecting" screen.
 */
class WebActivity : Activity() {
    var serverUrl: String? = null
        private set

    private lateinit var container: FrameLayout
    private lateinit var errorView: View
    private lateinit var errorBody: TextView
    private lateinit var volume: VolumeSync
    private var webView: WebView? = null
    private var customView: View? = null
    private var customViewCallback: WebChromeClient.CustomViewCallback? = null

    /** The page's current address (client-side navigations included), restored after a renderer loss. */
    private var lastUrl: String? = null

    /** Origin of the page now showing, read by the bridge from WebView threads. */
    @Volatile
    private var pageOrigin: String? = null

    /** The current navigation failed; reset by every new navigation, so a later success clears it. */
    private var navigationFailed = false
    private var lastError: String = ""
    private var retryAttempt = 0
    private var clearHistoryOnLoad = false
    private var rebuildOnResume = false
    private var resumed = false

    private val backPress = PressTracker()
    private val menuPress = PressTracker()
    private val main = Handler(Looper.getMainLooper())
    private val retry = Runnable { if (errorView.visibility == View.VISIBLE) reload(manual = false) }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val server = ServerStore.get(this)
        if (server == null) {
            startActivity(Intent(this, ServerActivity::class.java))
            finish()
            return
        }
        serverUrl = server
        setContentView(R.layout.activity_web)
        container = findViewById(R.id.web_container)
        errorView = findViewById(R.id.error)
        errorBody = findViewById(R.id.error_body)
        findViewById<Button>(R.id.retry).setOnClickListener { reload(manual = true) }
        findViewById<Button>(R.id.change_server).setOnClickListener { openServerSettings() }
        volume = VolumeSync(this) { state ->
            evaluate("window.dispatchEvent(new CustomEvent('librarian-tv-volume',{detail:${state.toJson()}}))")
        }

        if (BuildConfig.DEBUG) WebView.setWebContentsDebuggingEnabled(true)
        val restored = savedInstanceState?.getString(STATE_URL)?.takeIf { ServerAddress.sameOrigin(it, server) }
        buildWebView()
        load(restored ?: server)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        if (!intent.getBooleanExtra(EXTRA_SERVER_CHANGED, false)) return
        val server = ServerStore.get(this) ?: return
        if (server == serverUrl && webView != null) return
        serverUrl = server
        lastUrl = null
        clearHistoryOnLoad = true
        if (webView == null) buildWebView()
        load(server)
    }

    override fun onSaveInstanceState(outState: Bundle) {
        super.onSaveInstanceState(outState)
        lastUrl?.let { outState.putString(STATE_URL, it) }
    }

    override fun onStart() {
        super.onStart()
        if (::volume.isInitialized) volume.start()
    }

    override fun onStop() {
        if (::volume.isInitialized) volume.stop()
        super.onStop()
    }

    override fun onResume() {
        super.onResume()
        resumed = true
        webView?.resumeTimers()
        webView?.onResume()
        if (rebuildOnResume) {
            rebuildOnResume = false
            buildWebView()
            load(lastUrl ?: serverUrl ?: return)
        }
    }

    override fun onPause() {
        resumed = false
        webView?.onPause()
        webView?.pauseTimers()
        CookieManager.getInstance().flush()
        super.onPause()
    }

    override fun onDestroy() {
        main.removeCallbacks(retry)
        destroyWebView()
        super.onDestroy()
    }

    override fun onTrimMemory(level: Int) {
        super.onTrimMemory(level)
        @Suppress("DEPRECATION")
        when {
            // Running but short of memory: let the page drop what it can refetch.
            level == ComponentCallbacks2.TRIM_MEMORY_RUNNING_LOW || level == ComponentCallbacks2.TRIM_MEMORY_RUNNING_CRITICAL ->
                evaluate("window.librarianTv && window.librarianTv.memoryPressure && window.librarianTv.memoryPressure()")
            // In the background and about to be killed: give the renderer back and rebuild on return.
            level >= ComponentCallbacks2.TRIM_MEMORY_MODERATE && !resumed && webView != null -> {
                Log.i(TAG, "Releasing the WebView in the background (trim level $level)")
                destroyWebView()
                rebuildOnResume = true
            }
        }
    }

    // ── The WebView ────────────────────────────────────────────────────────────────────────────

    @SuppressLint("SetJavaScriptEnabled")
    private fun buildWebView() {
        destroyWebView()
        // On the Activity itself: a WebView on any other context cannot find its window (it lays
        // out at 0 × 0 on a bare configuration context), and its density cannot be overridden.
        val view = WebView(this)
        view.layoutParams = FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT)
        view.setBackgroundColor(Color.parseColor("#0E0E19"))
        view.isFocusable = true
        view.isFocusableInTouchMode = true
        view.settings.apply {
            javaScriptEnabled = true
            domStorageEnabled = true // localStorage; IndexedDB is always on with JavaScript.
            mediaPlaybackRequiresUserGesture = false
            cacheMode = WebSettings.LOAD_DEFAULT // the server's cache headers decide
            useWideViewPort = true // honour the viewport width TvCanvas sets
            loadWithOverviewMode = true
            textZoom = 100 // the page sizes its own text; a system font scale must not double it
            setSupportZoom(false)
            builtInZoomControls = false
            displayZoomControls = false
            allowFileAccess = false
            allowContentAccess = false
            javaScriptCanOpenWindowsAutomatically = false
            setSupportMultipleWindows(false)
            userAgentString = "$userAgentString LibrarianTV/${BuildConfig.VERSION_NAME}"
        }
        CookieManager.getInstance().apply {
            setAcceptCookie(true)
            setAcceptThirdPartyCookies(view, false)
        }
        // Lets Android reclaim the renderer while the app is hidden; onRenderProcessGone rebuilds it.
        if (Build.VERSION.SDK_INT >= 26) view.setRendererPriorityPolicy(WebView.RENDERER_PRIORITY_IMPORTANT, true)
        view.addJavascriptInterface(TvBridge(this), "LibrarianTV")
        view.webViewClient = Client()
        view.webChromeClient = Chrome()
        container.addView(view)
        // Timers are paused for every WebView in onPause; a WebView built since must restart them.
        view.resumeTimers()
        view.requestFocus()
        webView = view
    }

    private fun destroyWebView() {
        val view = webView ?: return
        webView = null
        keepScreenOn(false)
        container.removeView(view)
        view.stopLoading()
        view.destroy()
    }

    private fun load(url: String) {
        webView?.loadUrl(url)
    }

    private fun reload(manual: Boolean) {
        main.removeCallbacks(retry)
        if (manual) retryAttempt = 0
        if (webView == null) buildWebView()
        load(lastUrl ?: serverUrl ?: return)
    }

    /** The page's viewport: the 10-foot canvas at this TV's display size. */
    private fun applyViewport() {
        evaluate(TvCanvas.viewportScript(TvCanvas.viewportWidth(DisplayPrefs.size(this))))
    }

    /** "Reconnecting to …": retried automatically, sooner at first (2, 4, 8, 16, then every 30 s). */
    private fun showReconnecting() {
        val delay = RetryBackoff.delayMs(retryAttempt++)
        val reason = lastError.takeIf { it.isNotBlank() }?.let { " ($it)" } ?: ""
        errorBody.text = getString(R.string.load_failed_body, serverUrl ?: "", (delay / 1000).toString()) + reason
        if (errorView.visibility != View.VISIBLE) {
            errorView.visibility = View.VISIBLE
            findViewById<Button>(R.id.retry).requestFocus()
        }
        main.removeCallbacks(retry)
        main.postDelayed(retry, delay)
    }

    private fun hideReconnecting() {
        main.removeCallbacks(retry)
        retryAttempt = 0
        if (errorView.visibility == View.VISIBLE) {
            errorView.visibility = View.GONE
            webView?.requestFocus()
        }
    }

    private fun evaluate(script: String, callback: ((String?) -> Unit)? = null) {
        val view = webView
        if (view == null) {
            callback?.invoke(null)
            return
        }
        view.evaluateJavascript(script) { callback?.invoke(it) }
    }

    // ── Called by the bridge ───────────────────────────────────────────────────────────────────

    fun isServerPage(): Boolean = pageOrigin != null && pageOrigin == ServerAddress.originOf(serverUrl)

    fun volumeState(): VolumeState = volume.state()

    fun setVolumeLevel(level: Double) = volume.setLevel(level)

    fun setVolumeMuted(muted: Boolean): Boolean = volume.setMuted(muted)

    fun keepScreenOn(on: Boolean) {
        if (on) window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        else window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
    }

    fun openServerSettings() {
        startActivity(Intent(this, ServerActivity::class.java))
    }

    /** Applied at once: Chromium relays the page out for a changed viewport, keeping its place. */
    fun changeDisplaySize(percent: Int) {
        DisplayPrefs.setSize(this, percent)
        applyViewport()
    }

    // ── Remote keys ────────────────────────────────────────────────────────────────────────────

    override fun dispatchKeyEvent(event: KeyEvent): Boolean {
        val action = RemoteKeys.actionFor(event.keyCode) ?: return super.dispatchKeyEvent(event)
        if (errorView.visibility == View.VISIBLE) {
            // The reconnecting screen is native: its buttons take OK; Back leaves, Menu changes server.
            if (action == RemoteAction.Menu && event.action == KeyEvent.ACTION_UP) openServerSettings()
            return if (action == RemoteAction.Back) super.dispatchKeyEvent(event) else true
        }
        when (action) {
            RemoteAction.Back -> handleHeld(event, backPress, ::back)
            RemoteAction.Menu -> handleHeld(event, menuPress) {
                evaluate("window.librarianTv && window.librarianTv.menu && window.librarianTv.menu()")
            }
            is RemoteAction.PageKey -> when (event.action) {
                KeyEvent.ACTION_DOWN -> if (event.repeatCount == 0 || action.repeats) forwardKey(action.key, "keydown", event.repeatCount > 0)
                KeyEvent.ACTION_UP -> forwardKey(action.key, "keyup", false)
            }
        }
        return true
    }

    /** Short press: [onShort]. Held: the server screen. */
    private fun handleHeld(event: KeyEvent, tracker: PressTracker, onShort: () -> Unit) {
        when (event.action) {
            KeyEvent.ACTION_DOWN -> if (tracker.down(event.eventTime, event.repeatCount, event.isLongPress)) openServerSettings()
            KeyEvent.ACTION_UP -> when (tracker.up(event.eventTime, event.isCanceled)) {
                PressTracker.Release.SHORT -> onShort()
                PressTracker.Release.LONG -> openServerSettings()
                PressTracker.Release.IGNORED -> Unit
            }
        }
    }

    private fun back() {
        if (customView != null) {
            customViewCallback?.onCustomViewHidden()
            return
        }
        val view = webView ?: return finish()
        evaluate(BACK_SCRIPT) { result ->
            when (result?.trim('"')) {
                "handled" -> Unit
                "exit" -> finish()
                // An older web app without the bridge, or a page that did not take the key.
                else -> if (view.canGoBack()) view.goBack() else finish()
            }
        }
    }

    /** A remote button as a standard KeyboardEvent at the page's focused element. */
    private fun forwardKey(key: String, type: String, repeat: Boolean) {
        evaluate(
            "(function(k,t,r){var b=window.librarianTv;if(b&&b.keyEvent){b.keyEvent(k,t,r);return}" +
                "var e=document.activeElement||document.body;" +
                "e.dispatchEvent(new KeyboardEvent(t,{key:k,repeat:r,bubbles:true,cancelable:true}))})" +
                "(${JSONObject.quote(key)},${JSONObject.quote(type)},$repeat)",
        )
    }

    // ── Clients ────────────────────────────────────────────────────────────────────────────────

    private inner class Client : WebViewClient() {
        override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
            val url = request.url.toString()
            if (ServerAddress.sameOrigin(url, serverUrl)) return false
            // Everything else leaves the app (if the TV can open it at all).
            if (request.isForMainFrame && (request.url.scheme == "http" || request.url.scheme == "https")) {
                try {
                    startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)))
                } catch (_: ActivityNotFoundException) {
                    Log.i(TAG, "No app opens external links")
                }
            }
            return true
        }

        override fun onPageStarted(view: WebView, url: String?, favicon: Bitmap?) {
            navigationFailed = false
            pageOrigin = ServerAddress.originOf(url)
            // As early as possible, to keep the phone-sized first layout from flashing.
            applyViewport()
        }

        override fun doUpdateVisitedHistory(view: WebView, url: String?, isReload: Boolean) {
            if (ServerAddress.sameOrigin(url, serverUrl)) lastUrl = url
        }

        override fun onPageFinished(view: WebView, url: String?) {
            pageOrigin = ServerAddress.originOf(view.url ?: url)
            applyViewport()
            if (navigationFailed) {
                showReconnecting()
            } else {
                // A load that succeeds always clears the screen, however it was reached.
                hideReconnecting()
                // The current volume, for a page that missed the events before it loaded.
                if (::volume.isInitialized) {
                    evaluate("window.dispatchEvent(new CustomEvent('librarian-tv-volume',{detail:${volume.state().toJson()}}))")
                }
            }
            if (clearHistoryOnLoad && !navigationFailed) {
                clearHistoryOnLoad = false
                view.clearHistory()
            }
        }

        override fun onReceivedError(view: WebView, request: WebResourceRequest, error: WebResourceError) {
            if (!request.isForMainFrame) return
            navigationFailed = true
            lastError = error.description?.toString() ?: ""
        }

        override fun onRenderProcessGone(view: WebView, detail: RenderProcessGoneDetail): Boolean {
            Log.w(TAG, "WebView renderer gone (crashed: ${detail.didCrash()}); rebuilding")
            if (view != webView) return true
            destroyWebView()
            if (resumed) {
                buildWebView()
                load(lastUrl ?: serverUrl ?: return true)
            } else {
                rebuildOnResume = true
            }
            return true
        }
    }

    private inner class Chrome : WebChromeClient() {
        override fun onShowCustomView(view: View, callback: CustomViewCallback) {
            if (customView != null) {
                callback.onCustomViewHidden()
                return
            }
            customView = view
            customViewCallback = callback
            (window.decorView as ViewGroup).addView(view, FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT))
        }

        override fun onHideCustomView() {
            customView?.let { (window.decorView as ViewGroup).removeView(it) }
            customView = null
            customViewCallback = null
            webView?.requestFocus()
        }

        /** No grey play-button poster before a video's first frame. */
        override fun getDefaultVideoPoster(): Bitmap = Bitmap.createBitmap(1, 1, Bitmap.Config.ARGB_8888)

        override fun onPermissionRequest(request: PermissionRequest) = request.deny()
    }

    companion object {
        const val EXTRA_SERVER_CHANGED = "server_changed"
        private const val STATE_URL = "url"
        private const val TAG = "LibrarianTV"
        private const val BACK_SCRIPT =
            "(function(){try{var t=window.librarianTv;return t&&t.back?t.back():'missing'}catch(e){return 'unhandled'}})()"
    }
}

/** Reconnect delays: 2, 4, 8, 16 s, then every 30 s (RetryBackoffTest). */
object RetryBackoff {
    fun delayMs(attempt: Int): Long = if (attempt >= 4) 30_000L else 2_000L shl attempt.coerceAtLeast(0)
}
