import { clearTokens, getSession, isTokenExpired, setTokens, type AuthSession } from "./auth";
import { print } from "graphql";
import type { TypedDocumentNode } from "@apollo/client";
import { API_BASE_URL } from "./api/baseUrl";
import { MeDocument, RefreshTokenDocument } from "./graphql/generated/graphql";

// Use a separate transport so protected requests can await renewal without
// recursively entering Apollo's authentication link. Credentials stay HttpOnly.
async function authRequest<T, V>(document: TypedDocumentNode<T, V>): Promise<T> {
  const response = await fetch(`${API_BASE_URL}/graphql`, {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ query: print(document) }),
    signal: AbortSignal.timeout(15000),
  });
  if (!response.ok) throw new Error(`Session renewal unavailable (${response.status})`);
  const result = await response.json();
  if (result.errors?.length || !result.data) {
    throw new Error("Session renewal unavailable; will retry");
  }
  return result.data as T;
}

// Refresh tokens rotate. Share one request across hooks/StrictMode mounts and
// serialize tabs so a second request cannot revoke a freshly renewed session.
let pendingRefresh: Promise<AuthSession | null> | null = null;

/** Renew before protected requests, including the first request after sleep. */
export async function ensureFreshSession(): Promise<void> {
  if (!getSession() || !isTokenExpired()) return;
  if (!(await refreshSession())) {
    clearTokens();
    throw new Error("Session expired; sign in again");
  }
}

export function refreshSession(): Promise<AuthSession | null> {
  if (pendingRefresh) return pendingRefresh;
  const initialExpiry = getSession()?.expiresAt;
  const performRefresh = async (): Promise<AuthSession | null> => {
    const current = getSession();
    if (
      current &&
      current.expiresAt !== initialExpiry &&
      current.expiresAt > Date.now() / 1000 + 30
    ) {
      return current;
    }
    const result = await authRequest(RefreshTokenDocument);
    const payload = result.refreshToken;
    if (payload?.success === false) return null;
    if (!payload?.tokens) throw new Error("Incomplete session renewal response");
    let user = getSession()?.user;
    if (!user) {
      const result = await authRequest(MeDocument);
      const me = result.me;
      if (me)
        user = {
          id: me.id,
          username: me.username,
          role: me.role,
          email: me.email ?? undefined,
          displayName: me.displayName ?? undefined,
        };
    }
    if (!user) throw new Error("Unable to restore session user");
    const session = {
      user,
      expiresAt: Math.floor(Date.now() / 1000) + payload.tokens.expiresIn,
    };
    setTokens(session, { refresh: true });
    return session;
  };
  pendingRefresh = (
    typeof navigator !== "undefined" && navigator.locks
      ? navigator.locks.request("librarian-session-refresh", performRefresh)
      : performRefresh()
  ).finally(() => {
    pendingRefresh = null;
  });
  return pendingRefresh;
}
