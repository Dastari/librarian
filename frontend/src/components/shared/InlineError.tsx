import { IconAlertTriangle } from '@tabler/icons-react'

interface InlineErrorProps {
  /** The error message to display */
  message: string
  /** Optional className for additional styling */
  className?: string
  /** Whether to show an icon (default: false for compact display) */
  showIcon?: boolean
}

/**
 * Standardized inline error component for forms and modals.
 * Use this for displaying validation errors, API errors, etc. within forms.
 *
 * @example
 * ```tsx
 * {error && <InlineError message={error} />}
 * ```
 */
export function InlineError({ message, className = '', showIcon = false }: InlineErrorProps) {
  return (
    <div
      className={`bg-danger-50 text-danger-600 px-4 py-2 rounded-lg text-sm ${className}`}
      role="alert"
    >
      {showIcon ? (
        <div className="flex items-center gap-2">
          <IconAlertTriangle size={16} className="shrink-0" />
          <span>{message}</span>
        </div>
      ) : (
        message
      )}
    </div>
  )
}
