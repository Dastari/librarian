package net.dastari.librarian.tv

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class DiscoveredServerTest {
    private fun txt(vararg pairs: Pair<String, String>): Map<String, ByteArray?> =
        pairs.associate { (key, value) -> key to value.toByteArray() }

    @Test
    fun theAnnouncedUrlWins() {
        val server = DiscoveredServer.fromRecord(
            "Librarian on media-box",
            "192.168.1.20",
            3000,
            txt("url" to "https://media.example.com/", "name" to "media-box", "version" to "0.5.0", "txtvers" to "1"),
        )!!
        assertEquals("https://media.example.com", server.url)
        assertEquals("media-box", server.name)
        assertEquals("0.5.0", server.version)
        assertEquals("Librarian on media-box", server.title)
    }

    @Test
    fun withoutAUsableUrlTheSrvAddressIsUsed() {
        assertEquals("http://192.168.1.20:3001", DiscoveredServer.fromRecord("Librarian", "/192.168.1.20", 3001, txt("url" to "not a url"))!!.url)
        assertEquals("http://[fe80::1]:3001", DiscoveredServer.fromRecord("Librarian", "fe80::1%eth0", 3001, emptyMap())!!.url)
        assertNull(DiscoveredServer.fromRecord("Librarian", null, 3001, emptyMap()))
        assertNull(DiscoveredServer.fromRecord("Librarian", "192.168.1.20", 0, mapOf("url" to null)))
    }

    @Test
    fun serversAreListedOncePerAddress() {
        val a = DiscoveredServer("Librarian on b", "http://10.0.0.2:3001", "b", null)
        val b = DiscoveredServer("Librarian on a", "http://10.0.0.1:3001", "a", null)
        val duplicate = a.copy(serviceName = "Librarian on b (2)")
        assertEquals(listOf(b, a), DiscoveredServer.merge(listOf(a, b, duplicate)))
    }
}
