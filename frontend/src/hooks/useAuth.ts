import { refreshSession } from "../lib/refreshSession";
import { useState, useEffect, useCallback } from "react";
import {
  LoginDocument,
  RegisterDocument,
  LogoutDocument,
  MeDocument,
} from "../lib/graphql/generated/graphql";
import {
  type AuthUser,
  type AuthSession,
  getSession,
  setTokens,
  clearTokens,
} from "../lib/auth";
import { apolloClient } from "../lib/graphql/client";

// ============================================================================
// Hook
// ============================================================================

/**
 * Hook for authentication state and actions.
 * Use this in components that need user info or sign in/out functionality.
 *
 * This hook uses custom GraphQL-based authentication with cookie-backed session storage.
 */
export function useAuth() {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [session, setSession] = useState<AuthSession | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Refresh the access token using the refresh token
  const refreshAccessToken = useCallback(async (): Promise<boolean> => {
    try {
      const newSession = await refreshSession();
      if (newSession) {
        setSession(newSession);
        setUser(newSession.user);
        return true;
      }
    } catch (err) {
      console.warn("[Auth] Token refresh temporarily unavailable; will retry", err);
      return false;
    }

    // Refresh failed, clear everything
    clearTokens();
    setSession(null);
    setUser(null);
    return false;
  }, []);

  // Initialize auth state on mount - use stored session directly without server validation
  // Server validation is already done by main.tsx, so we just sync with stored state
  useEffect(() => {
    const syncAuthState = () => {
      try {
        // Check for existing session in cookies
        const existingSession = getSession();

        if (existingSession) {
          // Use stored session directly - main.tsx handles server validation
          setUser(existingSession.user);
          setSession(existingSession);
        } else {
          // No valid session
          setUser(null);
          setSession(null);
        }
      } catch (err) {
        console.error("[Auth] Sync error:", err);
        setError(err instanceof Error ? err.message : "Authentication error");
      } finally {
        setLoading(false);
      }
    };

    syncAuthState();
  }, []);

  // Note: Token refresh interval is handled by main.tsx
  // This hook just syncs with the stored session state

  // Listen for auth changes from other components (same-tab and cross-tab)
  useEffect(() => {
    const handleAuthChange = (data: { type: string }) => {
      if (data?.type === "login") {
        const newSession = getSession();
        if (newSession) {
          setSession(newSession);
          setUser(newSession.user);
          setLoading(false);
        }
      } else if (data?.type === "logout") {
        setSession(null);
        setUser(null);
        setLoading(false);
      }
    };

    // Same-tab listener
    const handleCustomEvent = (e: Event) => {
      handleAuthChange((e as CustomEvent).detail);
    };

    // Cross-tab listener via BroadcastChannel
    let authChannel: BroadcastChannel | null = null;
    try {
      authChannel = new BroadcastChannel("librarian-auth");
      authChannel.onmessage = (e) => handleAuthChange(e.data);
    } catch {
      // BroadcastChannel not supported
    }

    window.addEventListener("auth-change", handleCustomEvent);
    return () => {
      window.removeEventListener("auth-change", handleCustomEvent);
      authChannel?.close();
    };
  }, []);

  /**
   * Sign in with email and password.
   * @param email - The user's email address
   * @param password - The user's password
   */
  const signIn = async (email: string, password: string) => {
    setError(null);

    const result = await apolloClient.mutate({
      mutation: LoginDocument,
      variables: { input: { usernameOrEmail: email, password: password } },
    });

    if (result.error) {
      throw new Error(result.error.message || "Login failed");
    }

    const authData = result.data?.login;
    if (!authData?.success) {
      throw new Error(authData?.error ?? "Login failed");
    }

    if (!authData.tokens || !authData.user) {
      throw new Error("Invalid login response");
    }

    const verification = await apolloClient.query({
      query: MeDocument,
      fetchPolicy: "network-only",
    });
    if (
      !verification.data?.me ||
      verification.data.me.id !== authData.user.id
    ) {
      throw new Error(
        "Login succeeded, but the authenticated session cookie was not established. Rebuild and restart the backend so it matches the frontend.",
      );
    }

    const verifiedUser = verification.data.me;
    const authUser: AuthUser = {
      id: verifiedUser.id,
      email: verifiedUser.email ?? undefined,
      username: verifiedUser.username,
      role: verifiedUser.role,
      displayName: verifiedUser.displayName ?? undefined,
    };

    const newSession: AuthSession = {
      expiresAt: Math.floor(Date.now() / 1000) + authData.tokens.expiresIn,
      user: authUser,
    };

    setTokens(newSession);
    setSession(newSession);
    setUser(authUser);
  };

  /**
   * Sign up with email, name, and password.
   * @param email - The user's email address (required, used for login)
   * @param name - The user's full name (required)
   * @param password - The user's password (min 6 characters)
   * @param inviteToken - Invite code (required unless this is the first account on the server)
   */
  const signUp = async (
    email: string,
    name: string,
    password: string,
    inviteToken?: string,
  ) => {
    setError(null);

    const result = await apolloClient.mutate({
      mutation: RegisterDocument,
      variables: {
        input: {
          email: email,
          name: name,
          password: password,
          inviteToken: inviteToken || undefined,
        },
      },
    });

    if (result.error) {
      throw new Error(result.error.message || "Registration failed");
    }

    const reg = result.data?.register;
    if (!reg?.success) {
      throw new Error(reg?.error ?? "Registration failed");
    }

    if (!reg.tokens || !reg.user) {
      throw new Error("Invalid registration response");
    }

    const authUser: AuthUser = {
      id: reg.user.id,
      email: reg.user.email ?? undefined,
      username: reg.user.username,
      role: reg.user.role,
      displayName: reg.user.displayName || undefined,
    };

    const newSession: AuthSession = {
      expiresAt: Math.floor(Date.now() / 1000) + reg.tokens.expiresIn,
      user: authUser,
    };

    setTokens(newSession);
    setSession(newSession);
    setUser(authUser);
  };

  const signOut = async () => {
    try {
      await apolloClient.mutate({
        mutation: LogoutDocument,
      });
    } catch (err) {
      // Log but don't throw - we still want to clear local state
      console.error("[Auth] Logout mutation failed:", err);
    }

    // Always clear local state
    clearTokens();
    setSession(null);
    setUser(null);
  };

  return {
    user,
    session,
    loading,
    error,
    isAuthenticated: !!user,
    signIn,
    signUp,
    signOut,
    refreshToken: refreshAccessToken,
  };
}
