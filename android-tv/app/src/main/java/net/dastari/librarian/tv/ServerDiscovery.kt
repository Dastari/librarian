package net.dastari.librarian.tv

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.net.wifi.WifiManager
import android.os.Handler
import android.os.Looper
import android.util.Log

/**
 * Finds Librarian servers on the LAN with Android's own mDNS (NsdManager): `_librarian._tcp`
 * services are resolved one at a time (older Android refuses parallel resolves) and reported on
 * the main thread. Start in onStart, stop in onStop.
 */
class ServerDiscovery(
    context: Context,
    private val onChange: (List<DiscoveredServer>) -> Unit,
    private val onUnavailable: () -> Unit,
) {
    private val nsd = context.getSystemService(Context.NSD_SERVICE) as NsdManager?
    private val wifi = context.applicationContext.getSystemService(Context.WIFI_SERVICE) as WifiManager?
    private val main = Handler(Looper.getMainLooper())
    private val servers = LinkedHashMap<String, DiscoveredServer>()
    private val pending = ArrayDeque<NsdServiceInfo>()
    private var resolving = false
    private var running = false
    private var multicastLock: WifiManager.MulticastLock? = null

    private val discoveryListener = object : NsdManager.DiscoveryListener {
        override fun onDiscoveryStarted(serviceType: String) = Unit
        override fun onDiscoveryStopped(serviceType: String) = Unit
        override fun onStartDiscoveryFailed(serviceType: String, errorCode: Int) {
            Log.w(TAG, "mDNS discovery failed to start: $errorCode")
            main.post { running = false; onUnavailable() }
        }
        override fun onStopDiscoveryFailed(serviceType: String, errorCode: Int) = Unit
        override fun onServiceFound(info: NsdServiceInfo) {
            main.post {
                if (!running) return@post
                pending.addLast(info)
                resolveNext()
            }
        }
        override fun onServiceLost(info: NsdServiceInfo) {
            main.post {
                if (servers.remove(info.serviceName) != null) publish()
            }
        }
    }

    fun start() {
        val manager = nsd ?: return onUnavailable()
        if (running) return
        running = true
        try {
            multicastLock = wifi?.createMulticastLock("librarian-discovery")?.apply {
                setReferenceCounted(false)
                acquire()
            }
        } catch (error: RuntimeException) {
            Log.i(TAG, "No multicast lock: ${error.message}")
        }
        try {
            manager.discoverServices(DiscoveredServer.SERVICE_TYPE, NsdManager.PROTOCOL_DNS_SD, discoveryListener)
        } catch (error: RuntimeException) {
            Log.w(TAG, "mDNS discovery unavailable", error)
            running = false
            onUnavailable()
        }
    }

    fun stop() {
        if (!running) return
        running = false
        pending.clear()
        try {
            nsd?.stopServiceDiscovery(discoveryListener)
        } catch (_: RuntimeException) {
            // Already stopped.
        }
        try {
            multicastLock?.release()
        } catch (_: RuntimeException) {
        }
        multicastLock = null
    }

    @Suppress("DEPRECATION") // resolveService and host are deprecated on API 34+, and still work.
    private fun resolveNext() {
        val manager = nsd ?: return
        if (resolving || !running) return
        val info = pending.removeFirstOrNull() ?: return
        resolving = true
        try {
            manager.resolveService(info, object : NsdManager.ResolveListener {
                override fun onResolveFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {
                    main.post {
                        resolving = false
                        resolveNext()
                    }
                }

                override fun onServiceResolved(serviceInfo: NsdServiceInfo) {
                    val server = DiscoveredServer.fromRecord(
                        serviceInfo.serviceName ?: "",
                        serviceInfo.host?.hostAddress,
                        serviceInfo.port,
                        serviceInfo.attributes ?: emptyMap(),
                    )
                    main.post {
                        resolving = false
                        if (running && server != null) {
                            servers[server.serviceName] = server
                            publish()
                        }
                        resolveNext()
                    }
                }
            })
        } catch (error: RuntimeException) {
            Log.w(TAG, "Could not resolve ${info.serviceName}", error)
            resolving = false
        }
    }

    private fun publish() = onChange(DiscoveredServer.merge(servers.values))

    private companion object {
        const val TAG = "LibrarianDiscovery"
    }
}
