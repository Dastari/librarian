/**
 * Authentication utilities for custom GraphQL-based auth.
 * Manages JWT tokens in cookies for API authentication.
 * Cookies are shared across all tabs automatically.
 */

// ============================================================================
// Types
// ============================================================================

/** User information returned from auth endpoints */
export interface AuthUser {
  id: string;
  email?: string;
  username: string;
  role: string;
  displayName?: string;
}

/** Auth session containing tokens and user info */
export interface AuthSession {
  accessToken: string;
  refreshToken: string;
  expiresAt: number; // Unix timestamp in seconds
  user: AuthUser;
}

/** Auth tokens for API requests */
export interface AuthTokens {
  accessToken: string;
  refreshToken: string;
  expiresAt: number;
}

// ============================================================================
// Cookie Names
// ============================================================================

const COOKIE_NAMES = {
  ACCESS_TOKEN: "librarian_access_token",
  REFRESH_TOKEN: "librarian_refresh_token",
  EXPIRES_AT: "librarian_token_expires_at",
  USER: "librarian_user",
} as const;

// ============================================================================
// Cookie Utilities
// ============================================================================

interface CookieOptions {
  expires?: Date;
  path?: string;
  sameSite?: "Strict" | "Lax" | "None";
  secure?: boolean;
}

/** Set a cookie with the given name, value, and options */
function setCookie(
  name: string,
  value: string,
  options: CookieOptions = {},
): void {
  const {
    expires,
    path = "/",
    sameSite = "Lax",
    secure = window.location.protocol === "https:",
  } = options;

  let cookieString = `${encodeURIComponent(name)}=${encodeURIComponent(value)}`;

  if (expires) {
    cookieString += `; expires=${expires.toUTCString()}`;
  }

  cookieString += `; path=${path}`;
  cookieString += `; SameSite=${sameSite}`;

  if (secure) {
    cookieString += "; Secure";
  }

  // Debug logging in development
  if (import.meta.env?.DEV && name === COOKIE_NAMES.ACCESS_TOKEN) {
    console.debug(
      `[Cookie] Setting ${name}: ${value.substring(0, 20)}... (secure=${secure}, sameSite=${sameSite})`,
    );
  }

  document.cookie = cookieString;
}

/** Get a cookie value by name */
function getCookie(name: string): string | null {
  const nameEQ = encodeURIComponent(name) + "=";
  const cookies = document.cookie.split(";");

  // Debug logging in development
  if (import.meta.env?.DEV && name === COOKIE_NAMES.ACCESS_TOKEN) {
    console.debug(
      `[Cookie] Looking for "${name}", document.cookie has ${cookies.length} cookies`,
    );
  }

  for (const cookie of cookies) {
    let c = cookie.trim();
    if (c.indexOf(nameEQ) === 0) {
      const value = decodeURIComponent(c.substring(nameEQ.length));
      if (import.meta.env?.DEV && name === COOKIE_NAMES.ACCESS_TOKEN) {
        console.debug(`[Cookie] Found ${name}: ${value.substring(0, 20)}...`);
      }
      return value;
    }
  }

  return null;
}

/** Delete a cookie by name */
function deleteCookie(name: string): void {
  // Set cookie with expired date to delete it
  document.cookie = `${encodeURIComponent(name)}=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/`;
}

// ============================================================================
// Token Storage Functions
// ============================================================================

/** Get the current access token from cookies */
export function getAccessToken(): string | null {
  try {
    const token = getCookie(COOKIE_NAMES.ACCESS_TOKEN);
    // Only log when token is found (debug level) - no token is normal for unauthenticated users
    if (import.meta.env?.DEV && token) {
      console.debug(`[Auth] Access token found: ${token.substring(0, 30)}...`);
    }
    return token;
  } catch (e) {
    console.error("[Auth] Error reading access token:", e);
    return null;
  }
}

/** Get the current refresh token from cookies */
export function getRefreshToken(): string | null {
  try {
    return getCookie(COOKIE_NAMES.REFRESH_TOKEN);
  } catch {
    return null;
  }
}

/** Get the token expiration time (Unix timestamp in seconds) */
export function getTokenExpiresAt(): number | null {
  try {
    const expiresAt = getCookie(COOKIE_NAMES.EXPIRES_AT);
    return expiresAt ? parseInt(expiresAt, 10) : null;
  } catch {
    return null;
  }
}

/** Get the stored user info */
export function getStoredUser(): AuthUser | null {
  try {
    const userJson = getCookie(COOKIE_NAMES.USER);
    return userJson ? JSON.parse(userJson) : null;
  } catch {
    return null;
  }
}

/** Store tokens and user info in cookies */
export function setTokens(session: AuthSession): void {
  try {
    // Calculate expiry date from the token's expiresAt
    // Use refresh token expiry (typically 7 days) for cookie expiry
    const accessExpiry = new Date(session.expiresAt * 1000);
    // Refresh token cookies last longer (7 days from now)
    const refreshExpiry = new Date(Date.now() + 7 * 24 * 60 * 60 * 1000);

    setCookie(COOKIE_NAMES.ACCESS_TOKEN, session.accessToken, {
      expires: accessExpiry,
    });
    setCookie(COOKIE_NAMES.REFRESH_TOKEN, session.refreshToken, {
      expires: refreshExpiry,
    });
    setCookie(COOKIE_NAMES.EXPIRES_AT, session.expiresAt.toString(), {
      expires: refreshExpiry,
    });
    setCookie(COOKIE_NAMES.USER, JSON.stringify(session.user), {
      expires: refreshExpiry,
    });

    // Reset Apollo cache first, then notify listeners
    // This ensures the cache is ready before components try to refetch
    import("./graphql/client").then(({ resetApolloCache, restartWebSocket }) => {
      resetApolloCache();
      restartWebSocket();
      // Small delay to let the cache reset complete before triggering refetches
      setTimeout(() => {
        // Dispatch custom event for same-tab listeners
        window.dispatchEvent(
          new CustomEvent("auth-change", { detail: { type: "login" } }),
        );

        // Broadcast to other tabs
        try {
          new BroadcastChannel("librarian-auth").postMessage({ type: "login" });
        } catch {
          // BroadcastChannel not supported
        }
      }, 50);
    });
  } catch (error) {
    console.error("[Auth] Failed to store tokens:", error);
  }
}

/** Clear all auth data from cookies */
export function clearTokens(): void {
  try {
    deleteCookie(COOKIE_NAMES.ACCESS_TOKEN);
    deleteCookie(COOKIE_NAMES.REFRESH_TOKEN);
    deleteCookie(COOKIE_NAMES.EXPIRES_AT);
    deleteCookie(COOKIE_NAMES.USER);

    import("./graphql/client").then(({ resetApolloCache, restartWebSocket }) => {
      resetApolloCache();
      restartWebSocket();
      setTimeout(() => {
        window.dispatchEvent(
          new CustomEvent("auth-change", { detail: { type: "logout" } }),
        );
        try {
          new BroadcastChannel("librarian-auth").postMessage({ type: "logout" });
        } catch {
          // BroadcastChannel not supported
        }
      }, 50);
    });
  } catch (error) {
    console.error("[Auth] Failed to clear tokens:", error);
  }
}

/** Get the full session if valid tokens exist */
export function getSession(): AuthSession | null {
  const accessToken = getAccessToken();
  const refreshToken = getRefreshToken();
  const expiresAt = getTokenExpiresAt();
  const user = getStoredUser();

  if (!accessToken || !refreshToken || !expiresAt || !user) {
    return null;
  }

  return {
    accessToken,
    refreshToken,
    expiresAt,
    user,
  };
}

// ============================================================================
// Token Validation
// ============================================================================

/** Check if the access token is expired (with 30 second buffer) */
export function isTokenExpired(): boolean {
  const expiresAt = getTokenExpiresAt();
  if (!expiresAt) return true;

  // Add 30 second buffer to refresh before actual expiration
  const now = Math.floor(Date.now() / 1000);
  return now >= expiresAt - 30;
}

/** Check if we have a valid (non-expired) access token */
export function hasValidToken(): boolean {
  const accessToken = getAccessToken();
  return Boolean(accessToken) && !isTokenExpired();
}

// ============================================================================
// Auth Header Helpers
// ============================================================================

/** Get the Authorization header value (Bearer token) */
export function getAuthHeader(): string {
  const accessToken = getAccessToken();
  return accessToken ? `Bearer ${accessToken}` : "";
}

/** Get the Authorization header value synchronously (for WebSocket) */
export function getAuthHeaderSync(): string {
  return getAuthHeader();
}
