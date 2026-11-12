package net.dastari.librarian.tv

/**
 * A Librarian server announced on the LAN as `_librarian._tcp` (the backend's `lan_advertise`
 * service, docs/design.md Q138). Its TXT record carries `url` (the address to open), `name` and
 * `version`; without a usable `url` the SRV address and port are used.
 */
data class DiscoveredServer(
    val serviceName: String,
    val url: String,
    val name: String,
    val version: String?,
) {
    /** The line shown on its button. */
    val title: String get() = if (name.isNotBlank() && !serviceName.contains(name)) "$serviceName ($name)" else serviceName

    companion object {
        const val SERVICE_TYPE = "_librarian._tcp"

        fun fromRecord(serviceName: String, host: String?, port: Int, attributes: Map<String, ByteArray?>): DiscoveredServer? {
            fun text(key: String) = attributes[key]?.toString(Charsets.UTF_8)?.trim()?.takeIf { it.isNotEmpty() }
            val announced = text("url")?.let { ServerAddress.originOf(it) }
            val fallback = host?.trim()?.takeIf { it.isNotEmpty() && port in 1..65535 }
                ?.let { ServerAddress.origin("http", it.removePrefix("/").substringBefore('%'), port) }
            val url = announced ?: fallback ?: return null
            return DiscoveredServer(
                serviceName = serviceName.trim().ifEmpty { "Librarian" },
                url = url,
                name = text("name") ?: "",
                version = text("version"),
            )
        }

        /** One entry per address, sorted by name. */
        fun merge(servers: Collection<DiscoveredServer>): List<DiscoveredServer> =
            servers.distinctBy { it.url }.sortedBy { it.title.lowercase() }
    }
}
