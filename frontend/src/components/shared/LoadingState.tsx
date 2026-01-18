import { Spinner } from '@heroui/spinner'
import { Skeleton } from '@heroui/skeleton'

// ============================================================================
// Loading State Components
// ============================================================================

export interface LoadingStateProps {
  /** Optional loading message */
  message?: string
  /** Size of the spinner */
  size?: 'sm' | 'md' | 'lg'
}

/**
 * Full-area loading state with spinner and optional message.
 * Use this for page-level or section-level loading states.
 */
export function LoadingState({ message = 'Loading...', size = 'lg' }: LoadingStateProps) {
  return (
    <div className="flex flex-col items-center justify-center gap-4 py-12">
      <Spinner size={size} />
      {message && <p className="text-default-500">{message}</p>}
    </div>
  )
}

// ============================================================================
// Loading Skeleton Components
// ============================================================================

export interface LoadingSkeletonProps {
  /** Number of skeleton rows to render */
  rows?: number
  /** Height of each skeleton row */
  height?: string
}

/**
 * Skeleton loading state for lists/tables.
 * Use this when you want to show placeholder content during loading.
 */
export function LoadingSkeleton({ rows = 3, height = 'h-12' }: LoadingSkeletonProps) {
  return (
    <div className="space-y-3">
      {Array.from({ length: rows }).map((_, i) => (
        <Skeleton key={i} className={`${height} rounded-lg`} />
      ))}
    </div>
  )
}

export interface CardSkeletonProps {
  /** Number of cards to show */
  count?: number
}

/**
 * Grid of card skeletons for loading states in card views.
 */
export function CardSkeleton({ count = 6 }: CardSkeletonProps) {
  return (
    <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-6 gap-4">
      {Array.from({ length: count }).map((_, i) => (
        <div key={i} className="space-y-2">
          <Skeleton className="aspect-[2/3] rounded-lg" />
          <Skeleton className="h-4 w-3/4 rounded" />
          <Skeleton className="h-3 w-1/2 rounded" />
        </div>
      ))}
    </div>
  )
}

export interface TableSkeletonProps {
  /** Number of rows */
  rows?: number
  /** Number of columns */
  columns?: number
}

/**
 * Table skeleton for loading states in data tables.
 */
export function TableSkeleton({ rows = 5, columns = 4 }: TableSkeletonProps) {
  return (
    <div className="space-y-3">
      {/* Header */}
      <div className="flex gap-4">
        {Array.from({ length: columns }).map((_, i) => (
          <Skeleton key={i} className="h-8 flex-1 rounded" />
        ))}
      </div>
      {/* Rows */}
      {Array.from({ length: rows }).map((_, rowIdx) => (
        <div key={rowIdx} className="flex gap-4">
          {Array.from({ length: columns }).map((_, colIdx) => (
            <Skeleton key={colIdx} className="h-12 flex-1 rounded" />
          ))}
        </div>
      ))}
    </div>
  )
}

// ============================================================================
// Inline Loading Components
// ============================================================================

export interface InlineLoadingProps {
  /** Text to show next to the spinner */
  text?: string
}

/**
 * Small inline loading indicator for buttons or inline content.
 */
export function InlineLoading({ text }: InlineLoadingProps) {
  return (
    <span className="inline-flex items-center gap-2">
      <Spinner size="sm" />
      {text && <span className="text-default-500 text-sm">{text}</span>}
    </span>
  )
}
