package net.dastari.librarian.tv

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.view.KeyEvent
import android.view.View
import android.view.inputmethod.EditorInfo
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import java.net.HttpURLConnection
import java.net.URL
import kotlin.concurrent.thread

/**
 * The server screen: servers found on the network as buttons, and an address field for the rest.
 * Shown on first launch and when Back or Menu is held (or "Change server" in the web app's About).
 */
class ServerActivity : Activity() {
    private lateinit var status: TextView
    private lateinit var discovered: LinearLayout
    private lateinit var address: EditText
    private lateinit var connect: Button
    private lateinit var message: TextView
    private lateinit var useAnyway: Button
    private lateinit var discovery: ServerDiscovery
    private var anywayUrl: String? = null
    private var checking = false
    private var typed = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_server)
        status = findViewById(R.id.discovery_status)
        discovered = findViewById(R.id.discovered)
        address = findViewById(R.id.address)
        connect = findViewById(R.id.connect)
        message = findViewById(R.id.message)
        useAnyway = findViewById(R.id.use_anyway)

        ServerStore.get(this)?.let { current ->
            findViewById<TextView>(R.id.current).apply {
                text = getString(R.string.server_current, current)
                visibility = View.VISIBLE
            }
            address.setText(current)
        }
        address.setOnEditorActionListener { _, actionId, event ->
            val enter = event != null && event.keyCode == KeyEvent.KEYCODE_ENTER && event.action == KeyEvent.ACTION_DOWN
            if (actionId == EditorInfo.IME_ACTION_GO || actionId == EditorInfo.IME_ACTION_DONE || enter) {
                check()
                true
            } else {
                false
            }
        }
        address.setOnKeyListener { _, _, _ -> typed = true; false }
        connect.setOnClickListener { check() }
        useAnyway.setOnClickListener { anywayUrl?.let(::choose) }

        discovery = ServerDiscovery(this, ::showServers) {
            status.setText(R.string.server_discovery_failed)
        }
        status.postDelayed({
            if (discovered.childCount == 0 && !isFinishing) status.setText(R.string.server_found_none)
        }, NONE_FOUND_AFTER_MS)
        connect.requestFocus()
    }

    override fun onStart() {
        super.onStart()
        discovery.start()
    }

    override fun onStop() {
        discovery.stop()
        super.onStop()
    }

    private fun showServers(servers: List<DiscoveredServer>) {
        val hadNone = discovered.childCount == 0
        val focused = currentFocus
        discovered.removeAllViews()
        status.setText(if (servers.isEmpty()) R.string.server_found_none else R.string.server_found_some)
        for (server in servers) {
            val button = layoutInflater.inflate(R.layout.item_server, discovered, false) as Button
            button.text = "${server.title}\n${server.url}"
            button.setOnClickListener { choose(server.url) }
            discovered.addView(button)
        }
        // The first server found takes focus unless someone is already typing an address.
        if (hadNone && servers.isNotEmpty() && !typed && (focused == null || focused == connect)) {
            discovered.getChildAt(0).requestFocus()
        }
    }

    /** Try what was typed on each likely address; the first that answers is used. */
    private fun check() {
        if (checking) return
        val candidates = ServerAddress.candidates(address.text.toString())
        hideMessage()
        if (candidates.isEmpty()) {
            showMessage(getString(R.string.server_invalid), null)
            return
        }
        checking = true
        status.text = getString(R.string.server_checking, candidates.first())
        thread(name = "server-check") {
            val reachable = candidates.firstOrNull { answers(it) }
            runOnUiThread {
                checking = false
                if (isFinishing) return@runOnUiThread
                status.setText(if (discovered.childCount == 0) R.string.server_found_none else R.string.server_found_some)
                if (reachable != null) choose(reachable)
                else showMessage(getString(R.string.server_unreachable, candidates.first()), candidates.first())
            }
        }
    }

    private fun showMessage(text: String, anyway: String?) {
        message.text = text
        message.visibility = View.VISIBLE
        anywayUrl = anyway
        useAnyway.visibility = if (anyway != null) View.VISIBLE else View.GONE
        if (anyway != null) useAnyway.requestFocus()
    }

    private fun hideMessage() {
        message.visibility = View.GONE
        useAnyway.visibility = View.GONE
        anywayUrl = null
    }

    private fun choose(url: String) {
        ServerStore.set(this, url)
        startActivity(
            Intent(this, WebActivity::class.java)
                .addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP)
                .putExtra(WebActivity.EXTRA_SERVER_CHANGED, true),
        )
        finish()
    }

    private companion object {
        const val NONE_FOUND_AFTER_MS = 6_000L
        const val TIMEOUT_MS = 2_500

        /** Any HTTP answer below 500 means something is serving there. */
        fun answers(origin: String): Boolean = try {
            val connection = URL("$origin/").openConnection() as HttpURLConnection
            connection.connectTimeout = TIMEOUT_MS
            connection.readTimeout = TIMEOUT_MS
            connection.instanceFollowRedirects = true
            try {
                connection.responseCode in 100..499
            } finally {
                connection.disconnect()
            }
        } catch (_: Exception) {
            false
        }
    }
}
