/**
 * Authentication utilities for custom GraphQL-based auth.
 * Manages non-secret browser auth state. Access and refresh token values are
 * server-set HttpOnly cookies and are never read or written by this module.
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

export type RoleName = "admin" | "member";

export function normalizeRole(role: string | null | undefined): RoleName | null {
  const normalized = role?.trim().toLowerCase();
  if (normalized === "admin" || normalized === "member") {
    return normalized;
  }
  return null;
}

export function hasRole(
  user: Pick<AuthUser, "role"> | null | undefined,
  role: RoleName
): boolean {
  const userRole = normalizeRole(user?.role);
  return userRole === role || (role === "member" && userRole === "admin");
}

export function isAdmin(user: Pick<AuthUser, "role"> | null | undefined): boolean {
  return normalizeRole(user?.role) === "admin";
}

/** Auth session containing tokens and user info */
export interface AuthSession {
  expiresAt: number; // Unix timestamp in seconds
  user: AuthUser;
}

// ============================================================================
// Cookie Names
// ============================================================================

const COOKIE_NAMES = {
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
  options: CookieOptions = {}
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

  document.cookie = cookieString;
}

/** Get a cookie value by name */
function getCookie(name: string): string | null {
  const nameEQ = encodeURIComponent(name) + "=";
  const cookies = document.cookie.split(";");

  for (const cookie of cookies) {
    let c = cookie.trim();
    if (c.indexOf(nameEQ) === 0) {
      const value = decodeURIComponent(c.substring(nameEQ.length));
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

/** Store non-secret session timing and user display state. */
export function setTokens(session: AuthSession, options: { refresh?: boolean } = {}): void {
  try {
    const refreshExpiry = new Date(Date.now() + 30 * 24 * 60 * 60 * 1000);

    setCookie(COOKIE_NAMES.EXPIRES_AT, session.expiresAt.toString(), {
      // Retain refresh metadata after the access cookie expires (sleeping tabs).
      expires: refreshExpiry,
    });
    setCookie(COOKIE_NAMES.USER, JSON.stringify(session.user), {
      expires: refreshExpiry,
    });

    // Reset Apollo cache first, then notify listeners
    // This ensures the cache is ready before components try to refetch
    import("./graphql/client").then(
      ({ resetApolloCache, restartWebSocket }) => {
        if (!options.refresh) resetApolloCache();
        restartWebSocket();
        // Small delay to let the cache reset complete before triggering refetches
        setTimeout(() => {
          // Dispatch custom event for same-tab listeners
          window.dispatchEvent(
            new CustomEvent("auth-change", { detail: { type: "login" } })
          );

          // Broadcast to other tabs
          try {
            const channel = new BroadcastChannel("librarian-auth");
            channel.postMessage({ type: "login" });
            channel.close();
          } catch {
            // BroadcastChannel not supported
          }
        }, 50);
      }
    );
  } catch (error) {
    console.error("[Auth] Failed to store tokens:", error);
  }
}

/** Clear all auth data from cookies */
export function clearTokens(): void {
  try {
    deleteCookie(COOKIE_NAMES.EXPIRES_AT);
    deleteCookie(COOKIE_NAMES.USER);

    import("./graphql/client").then(
      ({ resetApolloCache, restartWebSocket }) => {
        resetApolloCache();
        restartWebSocket();
        setTimeout(() => {
          window.dispatchEvent(
            new CustomEvent("auth-change", { detail: { type: "logout" } })
          );
          try {
            const channel = new BroadcastChannel("librarian-auth");
            channel.postMessage({ type: "logout" });
            channel.close();
          } catch {
            // BroadcastChannel not supported
          }
        }, 50);
      }
    );
  } catch (error) {
    console.error("[Auth] Failed to clear tokens:", error);
  }
}

/** Get the full session if valid tokens exist */
export function getSession(): AuthSession | null {
  const expiresAt = getTokenExpiresAt();
  const user = getStoredUser();

  if (!user) {
    return null;
  }

  return {
    expiresAt: expiresAt ?? 0,
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
  return getStoredUser() !== null && !isTokenExpired();
}
