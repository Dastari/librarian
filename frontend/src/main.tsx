import { StrictMode, useMemo } from 'react'
import ReactDOM from 'react-dom/client'
import { RouterProvider, createRouter } from '@tanstack/react-router'
import { HeroUIProvider } from '@heroui/system'
import { NuqsAdapter } from 'nuqs/adapters/react'

// Import the generated route tree
import { routeTree } from './routeTree.gen'
import { ErrorBoundary } from './components/ErrorBoundary'
import { DEFAULT_AUTH_STATE } from './lib/auth-context'
import { useAuthState } from './hooks/useAuth'
import { initializeTheme } from './hooks/useTheme'

import './styles.css'
import reportWebVitals from './reportWebVitals.ts'

// Initialize theme immediately to prevent flash of wrong theme
initializeTheme()


// Create a new router instance with auth context
const router = createRouter({
  routeTree,
  context: {
    auth: DEFAULT_AUTH_STATE,
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
// Uses the consolidated useAuthState hook to avoid duplicate auth logic
function InnerApp() {
  const auth = useAuthState()

  // Memoize the context object to prevent unnecessary router refreshes
  const routerContext = useMemo(() => ({ auth }), [auth])

  // Show loading spinner while auth is initializing
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
          <NuqsAdapter>
            <InnerApp />
          </NuqsAdapter>
        </HeroUIProvider>
      </ErrorBoundary>
    </StrictMode>,
  )
}

// If you want to start measuring performance in your app, pass a function
// to log results (for example: reportWebVitals(console.log))
// or send to an analytics endpoint. Learn more: https://bit.ly/CRA-vitals
reportWebVitals()
