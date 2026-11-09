package net.dastari.librarian.tv

import java.net.URI

/**
 * Server addresses as people type them on a remote, and the origins the app loads. Pure Kotlin,
 * so it is unit-tested on the JVM (ServerAddressTest).
 */
object ServerAddress {
    /** A release build serves the web app on the backend's port; a development one on Vite's. */
    val DEFAULT_PORTS = listOf(3001, 3000)

    /**
     * The origins to try for what someone typed, best first. An address with a scheme or a port is
     * taken as written. A bare host ("192.168.1.20", "nas.local") is tried on Librarian's usual
     * ports and then on plain http. Paths are dropped: the web app always lives at the root.
     * Returns an empty list for something that is not an address.
     */
    fun candidates(input: String): List<String> {
        val raw = input.trim()
        if (raw.isEmpty() || raw.any { it.isWhitespace() }) return emptyList()
        val hasScheme = Regex("^[a-zA-Z][a-zA-Z0-9+.-]*://").containsMatchIn(raw)
        if (hasScheme && raw.substringAfter("://").trimEnd('/').isEmpty()) return emptyList()
        val text = if (hasScheme) raw else raw.trimEnd('/')
        if (text.isEmpty()) return emptyList()
        val withScheme = if (hasScheme) text else "http://${bracketBareIpv6(text)}"
        val uri = parse(withScheme) ?: return emptyList()
        val scheme = uri.scheme?.lowercase() ?: return emptyList()
        if (scheme != "http" && scheme != "https") return emptyList()
        val host = uri.host?.takeIf { it.isNotEmpty() } ?: return emptyList()
        if (hasScheme || uri.port != -1) return listOf(origin(scheme, host, uri.port))
        return DEFAULT_PORTS.map { origin("http", host, it) } + origin("http", host, -1)
    }

    /** The origin of a full URL (scheme, host and port, no path), or null when it is not http(s). */
    fun originOf(url: String?): String? {
        val uri = url?.let(::parse) ?: return null
        val scheme = uri.scheme?.lowercase()?.takeIf { it == "http" || it == "https" } ?: return null
        val host = uri.host?.takeIf { it.isNotEmpty() } ?: return null
        return origin(scheme, host, uri.port)
    }

    /** Whether a link stays on the server (and so stays in the app). */
    fun sameOrigin(url: String?, server: String?): Boolean {
        val a = originOf(url) ?: return false
        return a == originOf(server)
    }

    /** `http://host:port` with default ports dropped and IPv6 hosts bracketed. */
    fun origin(scheme: String, host: String, port: Int): String {
        val bracketed = if (host.contains(':') && !host.startsWith("[")) "[$host]" else host
        val defaultPort = (scheme == "http" && port == 80) || (scheme == "https" && port == 443)
        val portPart = if (port == -1 || defaultPort) "" else ":$port"
        return "$scheme://${bracketed.lowercase()}$portPart"
    }

    private fun parse(text: String): URI? = try {
        URI(text)
    } catch (_: Exception) {
        null
    }

    /** "fd00::1" typed without brackets: make it a URL host. "fd00::1:3001" is ambiguous and left alone. */
    private fun bracketBareIpv6(text: String): String {
        val hostPart = text.substringBefore('/')
        return if (hostPart.count { it == ':' } >= 2 && !hostPart.startsWith("[")) "[$hostPart]" + text.removePrefix(hostPart) else text
    }
}
