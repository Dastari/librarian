package net.dastari.librarian.tv

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ServerAddressTest {
    @Test
    fun aBareHostIsTriedOnLibrariansPortsThenPlainHttp() {
        assertEquals(
            listOf("http://192.168.1.20:3001", "http://192.168.1.20:3000", "http://192.168.1.20"),
            ServerAddress.candidates(" 192.168.1.20 "),
        )
        assertEquals(
            listOf("http://nas.local:3001", "http://nas.local:3000", "http://nas.local"),
            ServerAddress.candidates("NAS.local/"),
        )
    }

    @Test
    fun aPortOrSchemeIsTakenAsWritten() {
        assertEquals(listOf("http://192.168.1.20:3000"), ServerAddress.candidates("192.168.1.20:3000"))
        assertEquals(listOf("https://media.example.com"), ServerAddress.candidates("https://media.example.com/library/"))
        assertEquals(listOf("http://tv.lan"), ServerAddress.candidates("http://tv.lan:80"))
        assertEquals(listOf("https://tv.lan:8443"), ServerAddress.candidates("HTTPS://tv.lan:8443"))
    }

    @Test
    fun ipv6AddressesAreBracketed() {
        assertEquals(listOf("http://[fd00::7]:3001"), ServerAddress.candidates("[fd00::7]:3001"))
        assertEquals("http://[fd00::7]:3001", ServerAddress.candidates("fd00::7").first())
    }

    @Test
    fun nonsenseIsRefused() {
        assertTrue(ServerAddress.candidates("").isEmpty())
        assertTrue(ServerAddress.candidates("   ").isEmpty())
        assertTrue(ServerAddress.candidates("ftp://nas").isEmpty())
        assertTrue(ServerAddress.candidates("my server").isEmpty())
        assertTrue(ServerAddress.candidates("http://").isEmpty())
    }

    @Test
    fun linksStayInTheAppOnlyOnTheServersOrigin() {
        val server = "http://192.168.1.20:3000"
        assertTrue(ServerAddress.sameOrigin("http://192.168.1.20:3000/movies/1?x=1", server))
        assertFalse(ServerAddress.sameOrigin("http://192.168.1.20:3001/", server))
        assertFalse(ServerAddress.sameOrigin("https://www.themoviedb.org/movie/1", server))
        assertFalse(ServerAddress.sameOrigin("javascript:alert(1)", server))
        assertTrue(ServerAddress.sameOrigin("https://Media.Example.com:443/x", "https://media.example.com"))
        assertNull(ServerAddress.originOf("intent://x"))
    }
}
