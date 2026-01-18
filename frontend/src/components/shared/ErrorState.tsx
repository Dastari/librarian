import { Button } from '@heroui/button'
import { Card, CardBody } from '@heroui/card'
import { IconAlertTriangle, IconRefresh } from '@tabler/icons-react'

interface ErrorStateProps {
  /** The main error title (default: "Something went wrong") */
  title?: string
  /** The error message to display */
  message: string
  /** Callback for retry action */
  onRetry?: () => void
  /** Whether retry is currently loading */
  isRetrying?: boolean
  /** Optional className for the container */
  className?: string
}

/**
 * Standardized full-section error state component.
 * Use this for displaying errors that take up an entire section/tab/page area.
 *
 * @example
 * ```tsx
 * if (error) {
 *   return <ErrorState message={error} onRetry={refetch} />
 * }
 * ```
 */
export function ErrorState({
  title = 'Something went wrong',
  message,
  onRetry,
  isRetrying = false,
  className = '',
}: ErrorStateProps) {
  return (
    <div className={`w-full ${className}`}>
      <Card className="bg-danger-50/10 border-danger-200 border">
        <CardBody className="py-8 text-center">
          <IconAlertTriangle size={48} className="mx-auto mb-4 text-danger-400" />
          <h3 className="text-lg font-semibold mb-2">{title}</h3>
          <p className="text-default-500 mb-4">{message}</p>
          {onRetry && (
            <Button
              color="primary"
              variant="flat"
              onPress={onRetry}
              isLoading={isRetrying}
              startContent={!isRetrying && <IconRefresh size={16} />}
            >
              Retry
            </Button>
          )}
        </CardBody>
      </Card>
    </div>
  )
}
