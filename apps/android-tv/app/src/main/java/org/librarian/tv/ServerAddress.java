package org.librarian.tv;

import java.net.URI;
import java.net.URISyntaxException;
import java.util.Locale;

/** The hosted SPA currently requires a root origin, not a reverse-proxy subpath. */
final class ServerAddress {
    private ServerAddress() {}

    static String normalize(String input) {
        String value = input.trim();
        if (!value.contains(":")) {
            value = "https://" + value;
        } else if (!value.contains("://") && !value.matches("^[a-zA-Z][a-zA-Z0-9+.-]*:.*")) {
            value = "https://" + value;
        }
        try {
            URI uri = new URI(value);
            if (!isHttp(uri) || uri.getHost() == null || uri.getRawUserInfo() != null
                    || uri.getRawQuery() != null || uri.getRawFragment() != null
                    || (uri.getRawPath() != null && !uri.getRawPath().isEmpty() && !uri.getRawPath().equals("/"))
                    || uri.getPort() < -1 || uri.getPort() == 0 || uri.getPort() > 65535) {
                throw invalid();
            }
            String scheme = uri.getScheme().toLowerCase(Locale.ROOT);
            int port = uri.getPort();
            if (port == defaultPort(scheme)) port = -1;
            return new URI(scheme, null, uri.getHost().toLowerCase(Locale.ROOT), port, "/", null, null).toASCIIString();
        } catch (URISyntaxException exception) {
            throw invalid();
        }
    }

    static boolean sameOrigin(String server, String candidate) {
        try {
            URI expected = new URI(server);
            URI actual = new URI(candidate);
            return isHttp(actual) && actual.getHost() != null && actual.getRawUserInfo() == null
                    && expected.getScheme().equalsIgnoreCase(actual.getScheme())
                    && expected.getHost().equalsIgnoreCase(actual.getHost())
                    && port(expected) == port(actual);
        } catch (URISyntaxException exception) {
            return false;
        }
    }

    private static boolean isHttp(URI uri) {
        return "http".equalsIgnoreCase(uri.getScheme()) || "https".equalsIgnoreCase(uri.getScheme());
    }

    private static int port(URI uri) {
        return uri.getPort() == -1 ? defaultPort(uri.getScheme()) : uri.getPort();
    }

    private static int defaultPort(String scheme) {
        return "https".equalsIgnoreCase(scheme) ? 443 : 80;
    }

    private static IllegalArgumentException invalid() {
        return new IllegalArgumentException("Use an http:// or https:// server address, with no login details, path, query, or fragment.");
    }
}
