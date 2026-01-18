import { useRouter, type ErrorComponentProps } from '@tanstack/react-router'
import { Button } from '@heroui/button'
import { Card, CardBody } from '@heroui/card'
import { IconAlertTriangle } from '@tabler/icons-react'

/**
 * A reusable error component for route-level errors.
 * Shows the error message and provides retry/home actions.
 */
export function RouteError({ error, reset }: ErrorComponentProps) {
  const router = useRouter()

  return (
    <div className="flex items-center justify-center p-6 w-full">
      <Card className="max-w-lg w-full">
        <CardBody className="text-center space-y-4">
          <IconAlertTriangle size={48} className="text-danger-400 mx-auto" />
          <h1 className="text-xl font-bold text-danger">Something went wrong</h1>
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
    </div>
  )
}
