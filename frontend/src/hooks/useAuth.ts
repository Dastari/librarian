import { useState, useEffect, useCallback } from "react";
import { graphqlClient } from "../lib/graphql/client";
import {
  LoginDocument,
  RegisterDocument,
  RefreshTokenDocument,
  LogoutDocument,
} from "../lib/graphql/generated/graphql";
import {
  type AuthUser,
  type AuthSession,
  getSession,
  setTokens,
  clearTokens,
  getRefreshToken,
  hasValidToken,
} from "../lib/auth";

// ============================================================================
// Hook
// ============================================================================

/**
 * Hook for authentication state and actions.
 * Use this in components that need user info or sign in/out functionality.
 *
 * This hook uses custom GraphQL-based authentication with localStorage token storage.
 */
export function useAuth() {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [session, setSession] = useState<AuthSession | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Refresh the access token using the refresh token
  const refreshAccessToken = useCallback(async (): Promise<boolean> => {
    const refreshToken = getRefreshToken();
    if (!refreshToken) {
      return false;
    }

    try {
      const result = await graphqlClient
        .mutation(RefreshTokenDocument, {
          input: { RefreshToken: refreshToken },
        })
        .toPromise();

      const payload = result.data?.RefreshToken;
      if (payload?.Success && payload.Tokens) {
        const tokens = payload.Tokens;
        const existingSession = getSession();
        if (existingSession) {
          const newSession: AuthSession = {
            accessToken: tokens.AccessToken,
            refreshToken: tokens.RefreshToken,
            expiresAt: Date.now() + tokens.ExpiresIn * 1000,
            user: existingSession.user,
          };
          setTokens(newSession);
          setSession(newSession);
          return true;
        }
      }
    } catch (err) {
      console.error("[Auth] Token refresh failed:", err);
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

        if (existingSession && hasValidToken()) {
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

    const result = await graphqlClient
      .mutation(LoginDocument, {
        input: { UsernameOrEmail: email, Password: password },
      })
      .toPromise();

    if (result.error) {
      throw new Error(result.error.message || "Login failed");
    }

    const authData = result.data?.Login;
    if (!authData?.Success) {
      throw new Error(authData?.Error ?? "Login failed");
    }

    if (!authData.Tokens || !authData.User) {
      throw new Error("Invalid login response");
    }

    const authUser: AuthUser = {
      id: authData.User.Id,
      email: authData.User.Email ?? undefined,
      username: authData.User.Username,
      role: authData.User.Role,
      displayName: authData.User.DisplayName ?? undefined,
    };

    const newSession: AuthSession = {
      accessToken: authData.Tokens.AccessToken,
      refreshToken: authData.Tokens.RefreshToken,
      expiresAt: Date.now() + authData.Tokens.ExpiresIn * 1000,
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
   */
  const signUp = async (email: string, name: string, password: string) => {
    setError(null);

    const result = await graphqlClient
      .mutation(RegisterDocument, {
        input: {
          Email: email,
          Name: name,
          Password: password,
        },
      })
      .toPromise();

    if (result.error) {
      throw new Error(result.error.message || "Registration failed");
    }

    const reg = result.data?.Register;
    if (!reg?.Success) {
      throw new Error(reg?.Error ?? "Registration failed");
    }

    if (!reg.Tokens || !reg.User) {
      throw new Error("Invalid registration response");
    }

    const authUser: AuthUser = {
      id: reg.User.Id,
      email: reg.User.Email ?? undefined,
      username: reg.User.Username,
      role: reg.User.Role,
      displayName: reg.User.DisplayName || undefined,
    };

    const newSession: AuthSession = {
      accessToken: reg.Tokens.AccessToken,
      refreshToken: reg.Tokens.RefreshToken,
      expiresAt: Date.now() + reg.Tokens.ExpiresIn * 1000,
      user: authUser,
    };

    setTokens(newSession);
    setSession(newSession);
    setUser(authUser);
  };

  const signOut = async () => {
    const refreshToken = getRefreshToken();

    try {
      if (refreshToken) {
        await graphqlClient
          .mutation(LogoutDocument, {
            input: { RefreshToken: refreshToken },
          })
          .toPromise();
      }
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
    isAuthenticated: hasValidToken() && !!user,
    signIn,
    signUp,
    signOut,
    refreshToken: refreshAccessToken,
  };
}
