import { StrictMode, useState, useEffect, useMemo } from 'react'
import ReactDOM from 'react-dom/client'
import { RouterProvider, createRouter } from '@tanstack/react-router'
import { HeroUIProvider } from '@heroui/system'

// Import the generated route tree
import { routeTree } from './routeTree.gen'
import { ErrorBoundary } from './components/ErrorBoundary'
import { supabase, isSupabaseConfigured } from './lib/supabase'
import type { AuthContext } from './lib/auth-context'

import './styles.css'
import reportWebVitals from './reportWebVitals.ts'


// Create a new router instance with auth context
const router = createRouter({
  routeTree,
  context: {
    auth: {
      isAuthenticated: false,
      isLoading: true,
      session: null,
    } as AuthContext,
  },
  defaultPreload: 'intent',
  scrollRestoration: true,
  defaultStructuralSharing: true,
  defaultPreloadStaleTime: 0,
})

// Register the router instance for type safety
declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}

// Inner app component that manages auth state
function InnerApp() {
  const [auth, setAuth] = useState<AuthContext>({
    isAuthenticated: false,
    isLoading: true,
    session: null,
  })

  useEffect(() => {
    if (!isSupabaseConfigured) {
      setAuth({ isAuthenticated: false, isLoading: false, session: null })
      return
    }

    // Get initial session
    supabase.auth.getSession().then(({ data: { session } }) => {
      setAuth({
        isAuthenticated: !!session,
        isLoading: false,
        session,
      })
    })

    // Listen for auth changes - only update on meaningful events
    const { data: { subscription } } = supabase.auth.onAuthStateChange((event, session) => {
      // Skip token refresh events - they don't change auth status
      // and would cause unnecessary re-renders
      if (event === 'TOKEN_REFRESHED') {
        return
      }
      
      setAuth((prev) => {
        // Only update if authentication status actually changed
        const isAuthenticated = !!session
        if (prev.isAuthenticated === isAuthenticated && !prev.isLoading) {
          return prev // Return same reference to avoid re-render
        }
        return {
          isAuthenticated,
          isLoading: false,
          session,
        }
      })
    })

    return () => subscription.unsubscribe()
  }, [])

  // Memoize the context object to prevent unnecessary router refreshes
  const routerContext = useMemo(() => ({ auth }), [auth])

  // Show nothing while loading auth
  if (auth.isLoading) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
      </div>
    )
  }

  return <RouterProvider router={router} context={routerContext} />
}

// Render the app
const rootElement = document.getElementById('app')
if (rootElement && !rootElement.innerHTML) {
  const root = ReactDOM.createRoot(rootElement)
  root.render(
    <StrictMode>
      <ErrorBoundary>
        <HeroUIProvider>
          <InnerApp />
        </HeroUIProvider>
      </ErrorBoundary>
    </StrictMode>,
  )
}

// If you want to start measuring performance in your app, pass a function
// to log results (for example: reportWebVitals(console.log))
// or send to an analytics endpoint. Learn more: https://bit.ly/CRA-vitals
reportWebVitals()
