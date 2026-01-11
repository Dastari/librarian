import { useEffect, useRef } from 'react'
import { addToast } from '@heroui/toast'
import { onGraphQLError } from '../lib/graphql'

/**
 * Component that subscribes to GraphQL errors and displays them as toasts.
 * Mount this once at the app level to get global GraphQL error notifications.
 */
export function GraphQLErrorToaster() {
  // Track recent errors to prevent duplicates
  const recentErrors = useRef<Set<string>>(new Set())

  useEffect(() => {
    const unsubscribe = onGraphQLError(({ message, isNetworkError }) => {
      // Create a key for deduplication
      const errorKey = `${message}-${isNetworkError}`
      
      // Skip if we've shown this error recently
      if (recentErrors.current.has(errorKey)) {
        return
      }
      
      // Add to recent errors
      recentErrors.current.add(errorKey)
      
      // Remove from recent errors after 5 seconds
      setTimeout(() => {
        recentErrors.current.delete(errorKey)
      }, 5000)
      
      // Show toast
      addToast({
        title: isNetworkError ? 'Connection Error' : 'Error',
        description: message,
        color: 'danger',
        timeout: 5000,
      })
    })

    return () => {
      unsubscribe()
    }
  }, [])

  // This component doesn't render anything
  return null
}
