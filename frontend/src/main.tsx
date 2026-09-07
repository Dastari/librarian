import { startSessionRenewal } from "./lib/sessionRenewal";
import { refreshSession } from "./lib/refreshSession";
import { StrictMode, useState, useEffect, useMemo, useCallback } from "react";
import ReactDOM from "react-dom/client";
import { RouterProvider, createRouter } from "@tanstack/react-router";
import { ApolloProvider } from "@apollo/client/react";
import { HeroUIProvider } from "@heroui/system";
import { ToastProvider } from "@heroui/toast";
import { NuqsAdapter } from "nuqs/adapters/react";

// Import the generated route tree
import { routeTree } from "./routeTree.gen";
import { ErrorBoundary } from "./components/ErrorBoundary";
import type { AuthContext } from "./lib/auth-context";
import type { AuthUser } from "./lib/auth";
import {
  getSession,
  hasValidToken,
  isTokenExpired,
  clearTokens,
} from "./lib/auth";
import { apolloClient } from "./lib/graphql";
import {
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
    try {
      const newSession = await refreshSession();
      if (newSession) {
        setAuth({isAuthenticated: true, isLoading: false, session: newSession, user: newSession.user});
        return true;
      }
    } catch (err) {
      console.warn("[Auth] Token refresh temporarily unavailable; will retry", err);
      const existing = getSession();
      setAuth({ isAuthenticated: !!existing, isLoading: false, session: existing, user: existing?.user ?? null });
      return false;
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
        // Check for existing non-secret session state.
        const existingSession = getSession();

        if (!existingSession) {
          // HttpOnly cookies remain the source of truth after reopening the app,
          // even if the browser has lost its non-secret display metadata.
          await refreshAccessToken();
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
            const result = await apolloClient.query({
              query: MeDocument,
              fetchPolicy: "network-only",
            });
            if (result.data?.me) {
              const meUser = result.data.me;
              const authUser: AuthUser = {
                id: meUser.id,
                email: meUser.email || undefined,
                username: meUser.username,
                role: meUser.role,
                displayName: meUser.displayName || undefined,
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

  // Renew early during playback and immediately after sleep or reconnection.
  useEffect(() => startSessionRenewal(refreshAccessToken), [refreshAccessToken]);

  // Listen for auth changes from other components (e.g., SignInModal, Navbar signOut)
  useEffect(() => {
    // Handle same-tab auth changes (custom event from setTokens/clearTokens)
    const handleAuthChange = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      if (detail?.type === "login") {
        const newSession = getSession();
        if (newSession && hasValidToken()) {
          setAuth({
            isAuthenticated: true,
            isLoading: false,
            session: newSession,
            user: newSession.user,
          });
        }
      } else if (detail?.type === "logout") {
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
      authChannel = new BroadcastChannel("librarian-auth");
      authChannel.onmessage = (e) => {
        if (e.data?.type === "login") {
          const newSession = getSession();
          if (newSession && hasValidToken()) {
            setAuth({
              isAuthenticated: true,
              isLoading: false,
              session: newSession,
              user: newSession.user,
            });
          }
        } else if (e.data?.type === "logout") {
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
      console.warn(
        "[Auth] BroadcastChannel not supported, cross-tab sync disabled",
      );
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
        <ApolloProvider client={apolloClient}>
          <HeroUIProvider>
            <ToastProvider />
            <NuqsAdapter>
              <InnerApp />
            </NuqsAdapter>
          </HeroUIProvider>
        </ApolloProvider>
      </ErrorBoundary>
    </StrictMode>,
  );
}

// If you want to start measuring performance in your app, pass a function
// to log results (for example: reportWebVitals(console.log))
// or send to an analytics endpoint. Learn more: https://bit.ly/CRA-vitals
reportWebVitals();
