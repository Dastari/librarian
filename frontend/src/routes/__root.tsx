import { Outlet, createRootRouteWithContext, useRouter, type ErrorComponentProps } from '@tanstack/react-router'
import { TanStackRouterDevtoolsPanel } from '@tanstack/react-router-devtools'
import { TanStackDevtools } from '@tanstack/react-devtools'
import { Button } from '@heroui/button'
import { Card, CardBody } from '@heroui/card'
import { Navbar } from '../components/Navbar'
import { NotFound } from '../components/NotFound'
import { ErrorLogToaster } from '../components/ErrorLogToaster'
import { GraphQLErrorToaster } from '../components/GraphQLErrorToaster'
import type { AuthContext } from '../lib/auth-context'

interface RouterContext {
  auth: AuthContext
}

export const Route = createRootRouteWithContext<RouterContext>()({
  component: RootLayout,
  notFoundComponent: NotFound,
  errorComponent: RootErrorComponent,
})

function RootErrorComponent({ error, reset }: ErrorComponentProps) {
  const router = useRouter()

  return (
    <div className="min-h-screen bg-background text-foreground flex flex-col">
      <Navbar />
      <main className="flex grow items-center justify-center p-6">
        <Card className="max-w-lg w-full">
          <CardBody className="text-center space-y-4">
            <div className="text-5xl">💥</div>
            <h1 className="text-2xl font-bold text-danger">Something went wrong</h1>
            <p className="text-default-500">
              {error instanceof Error ? error.message : 'An unexpected error occurred'}
            </p>
            {import.meta.env.DEV && error instanceof Error && error.stack && (
              <details className="text-left mt-4">
                <summary className="cursor-pointer text-sm text-default-400 hover:text-default-600">
                  Stack trace (dev only)
                </summary>
                <pre className="mt-2 p-3 bg-default-100 rounded-lg text-xs overflow-auto max-h-64 text-left">
                  {error.stack}
                </pre>
              </details>
            )}
            <div className="flex gap-2 justify-center pt-4">
              <Button
                color="primary"
                onPress={() => {
                  reset()
                  router.invalidate()
                }}
              >
                Try Again
              </Button>
              <Button
                variant="flat"
                onPress={() => router.navigate({ to: '/' })}
              >
                Go Home
              </Button>
            </div>
          </CardBody>
        </Card>
      </main>
    </div>
  )
}

function RootLayout() {
  return (
    <div className="min-h-screen bg-background text-foreground flex flex-col">
      <Navbar />
      <main className="flex grow">
        <Outlet />
      </main>

      {/* Global error log toaster - shows toast for backend errors */}
      <ErrorLogToaster />

      {/* GraphQL error toaster - shows toast for GraphQL/network errors */}
      <GraphQLErrorToaster />

      {/* Dev tools - only in development */}
      {import.meta.env.DEV && (
        <TanStackDevtools
          config={{
            position: 'bottom-right',
          }}
          plugins={[
            {
              name: 'Tanstack Router',
              render: <TanStackRouterDevtoolsPanel />,
            },
          ]}
        />
      )}
    </div>
  )
}
