package org.librarian.tv;

import org.junit.Test;
import static org.junit.Assert.*;

public class ServerAddressTest {
    @Test public void normalizesRootOrigins() {
        assertEquals("https://library.example/", ServerAddress.normalize(" library.example "));
        assertEquals("https://library.example/", ServerAddress.normalize("HTTPS://LIBRARY.EXAMPLE:443/"));
        assertEquals("http://192.168.1.20:3000/", ServerAddress.normalize("http://192.168.1.20:3000"));
        assertEquals("http://[::1]:3000/", ServerAddress.normalize("http://[::1]:3000"));
    }

    @Test public void rejectsCredentialsPathsAndNonWebSchemes() {
        for (String input : new String[] { "", "file:///tmp/index.html", "javascript:alert(1)",
                "https://user:secret@library.example", "https://library.example/web", "https://library.example/%2f",
                "https://library.example/?token=secret", "https://library.example/#test", "https://library.example:0",
                "https://library.example:70000", "http://", "https://library.example\\@attacker.example" }) {
            assertThrows(input, IllegalArgumentException.class, () -> ServerAddress.normalize(input));
        }
    }

    @Test public void onlyAllowsSameOriginNavigation() {
        String origin = "https://library.example/";
        assertTrue(ServerAddress.sameOrigin(origin, "https://library.example:443/watch/123?position=10"));
        assertFalse(ServerAddress.sameOrigin(origin, "http://library.example/"));
        assertFalse(ServerAddress.sameOrigin(origin, "https://library.example:444/"));
        assertFalse(ServerAddress.sameOrigin(origin, "https://library.example.attacker.example/"));
        assertFalse(ServerAddress.sameOrigin(origin, "https://library.example@attacker.example/"));
        assertFalse(ServerAddress.sameOrigin(origin, "https://user@library.example/"));
        assertFalse(ServerAddress.sameOrigin(origin, "intent://library.example/"));
    }
}
