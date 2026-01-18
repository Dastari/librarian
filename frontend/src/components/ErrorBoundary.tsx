import { Component, type ReactNode } from 'react'
import { Button } from '@heroui/button'
import { Card, CardBody } from '@heroui/card'
import { IconAlertTriangle } from '@tabler/icons-react'

interface Props {
  children: ReactNode
  fallback?: ReactNode
}

interface State {
  hasError: boolean
  error: Error | null
}

/**
 * React error boundary component for catching render errors.
 * Displays a standardized error UI consistent with route-level errors.
 */
export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props)
    this.state = { hasError: false, error: null }
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error }
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error('Error caught by boundary:', error, errorInfo)
  }

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback
      }

      return (
        <div className="min-h-screen flex items-center justify-center p-6">
          <Card className="max-w-lg w-full">
            <CardBody className="text-center space-y-4">
              <IconAlertTriangle size={48} className="text-danger-400 mx-auto" />
              <h1 className="text-xl font-bold text-danger">Something went wrong</h1>
              <p className="text-default-500">
                {this.state.error?.message || 'An unexpected error occurred'}
              </p>
              {import.meta.env.DEV && this.state.error && (
                <details className="text-left mt-4">
                  <summary className="cursor-pointer text-sm text-default-400 hover:text-default-600">
                    Stack trace (dev only)
                  </summary>
                  <pre className="mt-2 p-3 bg-default-100 rounded-lg text-xs overflow-auto max-h-64 text-left">
                    {this.state.error.stack}
                  </pre>
                </details>
              )}
              <div className="flex gap-2 justify-center pt-4">
                <Button
                  color="primary"
                  onPress={() => window.location.reload()}
                >
                  Reload Page
                </Button>
                <Button
                  variant="flat"
                  onPress={() => this.setState({ hasError: false, error: null })}
                >
                  Try Again
                </Button>
              </div>
            </CardBody>
          </Card>
        </div>
      )
    }

    return this.props.children
  }
}
