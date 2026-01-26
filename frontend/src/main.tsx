import { StrictMode, useState, useEffect, useMemo, useCallback } from "react";
import ReactDOM from "react-dom/client";
import { RouterProvider, createRouter } from "@tanstack/react-router";
import { HeroUIProvider } from "@heroui/system";
import { NuqsAdapter } from "nuqs/adapters/react";

// Import the generated route tree
import { routeTree } from "./routeTree.gen";
import { ErrorBoundary } from "./components/ErrorBoundary";
import type { AuthContext } from "./lib/auth-context";
import type { AuthSession, AuthUser } from "./lib/auth";
import {
  getSession,
  hasValidToken,
  isTokenExpired,
  getRefreshToken,
  setTokens,
  clearTokens,
} from "./lib/auth";
import { graphqlClient } from "./lib/graphql";
import {
  RefreshTokenDocument,
  MeDocument,
} from "./lib/graphql/generated/graphql";
import { initializeTheme } from "./hooks/useTheme";

import "./styles.css";
import reportWebVitals from "./reportWebVitals.ts";

// Initialize theme immediately to prevent flash of wrong theme
initializeTheme();


// Create a new router instance with auth context
const router = createRouter({
  routeTree,
  context: {
    auth: {
      isAuthenticated: false,
      isLoading: true,
      session: null,
      user: null,
    } as AuthContext,
  },
  defaultPreload: "intent",
  scrollRestoration: true,
  defaultStructuralSharing: true,
  defaultPreloadStaleTime: 0,
});

// Register the router instance for type safety
declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

// Inner app component that manages auth state
function InnerApp() {
  const [auth, setAuth] = useState<AuthContext>({
    isAuthenticated: false,
    isLoading: true,
    session: null,
    user: null,
  });

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
          setAuth({
            isAuthenticated: true,
            isLoading: false,
            session: newSession,
            user: newSession.user,
          });
          return true;
        }
      }
    } catch (err) {
      console.error("[Auth] Token refresh failed:", err);
    }

    // Refresh failed, clear everything
    clearTokens();
    setAuth({
      isAuthenticated: false,
      isLoading: false,
      session: null,
      user: null,
    });
    return false;
  }, []);

  // Initialize auth state on mount
  useEffect(() => {
    const initAuth = async () => {
      try {
        // Check for existing session in localStorage
        const existingSession = getSession();

        if (!existingSession) {
          // No stored session
          setAuth({
            isAuthenticated: false,
            isLoading: false,
            session: null,
            user: null,
          });
          return;
        }

        // Check if token is expired
        if (isTokenExpired()) {
          // Try to refresh
          const refreshed = await refreshAccessToken();
          if (!refreshed) {
            // Refresh failed, state already updated in refreshAccessToken
            return;
          }
        } else {
          // Token is still valid, verify with server
          try {
            const result = await graphqlClient
              .query(MeDocument, {})
              .toPromise();

            if (result.data?.Me) {
              const meUser = result.data.Me;
              const authUser: AuthUser = {
                id: meUser.Id,
                email: meUser.Email || undefined,
                username: meUser.Username,
                role: meUser.Role,
                displayName: meUser.DisplayName || undefined,
              };
              setAuth({
                isAuthenticated: true,
                isLoading: false,
                session: existingSession,
                user: authUser,
              });
            } else {
              // Token was invalid, try refresh
              await refreshAccessToken();
            }
          } catch {
            // Server verification failed, try refresh
            await refreshAccessToken();
          }
        }
      } catch (err) {
        console.error("[Auth] Init error:", err);
        setAuth({
          isAuthenticated: false,
          isLoading: false,
          session: null,
          user: null,
        });
      }
    };

    initAuth();
  }, [refreshAccessToken]);

  // Set up token refresh interval
  useEffect(() => {
    if (!auth.session) return;

    // Check token expiration every minute
    const interval = setInterval(() => {
      if (isTokenExpired()) {
        refreshAccessToken();
      }
    }, 60000);

    return () => clearInterval(interval);
  }, [auth.session, refreshAccessToken]);

  // Listen for auth changes from other components (e.g., SignInModal, Navbar signOut)
  useEffect(() => {
    // Handle same-tab auth changes (custom event from setTokens/clearTokens)
    const handleAuthChange = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      if (detail?.type === 'login') {
        const newSession = getSession();
        if (newSession && hasValidToken()) {
          setAuth({
            isAuthenticated: true,
            isLoading: false,
            session: newSession,
            user: newSession.user,
          });
        }
      } else if (detail?.type === 'logout') {
        setAuth({
          isAuthenticated: false,
          isLoading: false,
          session: null,
          user: null,
        });
      }
    };

    // Handle cross-tab auth changes via BroadcastChannel
    // Cookies are shared across tabs, but we need to notify other tabs to update their state
    let authChannel: BroadcastChannel | null = null;
    try {
      authChannel = new BroadcastChannel('librarian-auth');
      authChannel.onmessage = (e) => {
        if (e.data?.type === 'login') {
          const newSession = getSession();
          if (newSession && hasValidToken()) {
            setAuth({
              isAuthenticated: true,
              isLoading: false,
              session: newSession,
              user: newSession.user,
            });
          }
        } else if (e.data?.type === 'logout') {
          setAuth({
            isAuthenticated: false,
            isLoading: false,
            session: null,
            user: null,
          });
        }
      };
    } catch {
      // BroadcastChannel not supported, fall back to no cross-tab sync
      console.warn('[Auth] BroadcastChannel not supported, cross-tab sync disabled');
    }

    window.addEventListener("auth-change", handleAuthChange);
    return () => {
      window.removeEventListener("auth-change", handleAuthChange);
      authChannel?.close();
    };
  }, []);

  // Memoize the context object to prevent unnecessary router refreshes
  const routerContext = useMemo(() => ({ auth }), [auth]);

  // Show nothing while loading auth
  if (auth.isLoading) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
      </div>
    );
  }

  return <RouterProvider router={router} context={routerContext} />;
}

// Render the app
const rootElement = document.getElementById("app");
if (rootElement && !rootElement.innerHTML) {
  const root = ReactDOM.createRoot(rootElement);
  root.render(
    <StrictMode>
      <ErrorBoundary>
        <HeroUIProvider>
          <NuqsAdapter>
            <InnerApp />
          </NuqsAdapter>
        </HeroUIProvider>
      </ErrorBoundary>
    </StrictMode>,
  );
}

// If you want to start measuring performance in your app, pass a function
// to log results (for example: reportWebVitals(console.log))
// or send to an analytics endpoint. Learn more: https://bit.ly/CRA-vitals
reportWebVitals();
