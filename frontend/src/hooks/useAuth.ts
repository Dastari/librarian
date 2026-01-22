import { useState, useEffect, useCallback } from "react";
import { graphqlClient } from "../lib/graphql/client";
import {
  LOGIN_MUTATION,
  REGISTER_MUTATION,
  REFRESH_TOKEN_MUTATION,
  LOGOUT_MUTATION,
} from "../lib/graphql";
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
// Types for GraphQL Responses
// ============================================================================

interface AuthTokens {
  accessToken: string;
  refreshToken: string;
  expiresIn: number;
  tokenType: string;
}

interface AuthUserResponse {
  id: string;
  email: string | null;
  username: string;
  role: string;
  displayName: string | null;
}

interface AuthResponse {
  success: boolean;
  error: string | null;
  user: AuthUserResponse | null;
  tokens: AuthTokens | null;
}

interface RefreshResponse {
  success: boolean;
  error: string | null;
  tokens: AuthTokens | null;
}

interface LogoutResponse {
  success: boolean;
  error: string | null;
}

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
        .mutation<{
          refreshToken: RefreshResponse;
        }>(REFRESH_TOKEN_MUTATION, { input: { refreshToken } })
        .toPromise();

      if (
        result.data?.refreshToken.success &&
        result.data.refreshToken.tokens
      ) {
        const tokens = result.data.refreshToken.tokens;
        // Keep existing user, just update tokens
        const existingSession = getSession();
        if (existingSession) {
          const newSession: AuthSession = {
            accessToken: tokens.accessToken,
            refreshToken: tokens.refreshToken,
            expiresAt: Date.now() + tokens.expiresIn * 1000,
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
      if (data?.type === 'login') {
        const newSession = getSession();
        if (newSession) {
          setSession(newSession);
          setUser(newSession.user);
          setLoading(false);
        }
      } else if (data?.type === 'logout') {
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
      authChannel = new BroadcastChannel('librarian-auth');
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
      .mutation<{ login: AuthResponse }>(LOGIN_MUTATION, {
        input: { usernameOrEmail: email, password },
      })
      .toPromise();

    if (result.error) {
      throw new Error(result.error.message || "Login failed");
    }

    const authData = result.data?.login;
    if (!authData?.success) {
      throw new Error(authData?.error || "Login failed");
    }

    if (!authData.tokens || !authData.user) {
      throw new Error("Invalid login response");
    }

    const authUser: AuthUser = {
      id: authData.user.id,
      email: authData.user.email || undefined,
      username: authData.user.username,
      role: authData.user.role,
      displayName: authData.user.displayName || undefined,
    };

    const newSession: AuthSession = {
      accessToken: authData.tokens.accessToken,
      refreshToken: authData.tokens.refreshToken,
      expiresAt: Date.now() + authData.tokens.expiresIn * 1000,
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
      .mutation<{ register: AuthResponse }>(REGISTER_MUTATION, {
        input: {
          email,
          name,
          password,
        },
      })
      .toPromise();

    if (result.error) {
      throw new Error(result.error.message || "Registration failed");
    }

    const authData = result.data?.register;
    if (!authData?.success) {
      throw new Error(authData?.error || "Registration failed");
    }

    if (!authData.tokens || !authData.user) {
      throw new Error("Invalid registration response");
    }

    const authUser: AuthUser = {
      id: authData.user.id,
      email: authData.user.email || undefined,
      username: authData.user.username,
      role: authData.user.role,
      displayName: authData.user.displayName || undefined,
    };

    const newSession: AuthSession = {
      accessToken: authData.tokens.accessToken,
      refreshToken: authData.tokens.refreshToken,
      expiresAt: Date.now() + authData.tokens.expiresIn * 1000,
      user: authUser,
    };

    setTokens(newSession);
    setSession(newSession);
    setUser(authUser);
  };

  const signOut = async () => {
    const refreshToken = getRefreshToken();
    
    try {
      // Call logout mutation to invalidate server-side session
      if (refreshToken) {
        await graphqlClient
          .mutation<{ logout: LogoutResponse }>(LOGOUT_MUTATION, {
            input: { refreshToken },
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
