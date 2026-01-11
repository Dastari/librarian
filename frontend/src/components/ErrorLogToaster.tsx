/**
 * ErrorLogToaster Component
 *
 * Listens for ERROR level log events via GraphQL subscription and displays them as toasts.
 * Should be mounted once in the app root to provide global error notifications.
 */

import { useEffect, useRef } from 'react'
import { addToast } from '@heroui/toast'
import {
  graphqlClient,
  ERROR_LOGS_SUBSCRIPTION,
  type LogEventSubscription,
} from '../lib/graphql'
import { useAuth } from '../hooks/useAuth'

// Debounce duplicate errors within this window (ms)
const DEBOUNCE_WINDOW = 5000

// Maximum message length for toast
const MAX_MESSAGE_LENGTH = 200

export function ErrorLogToaster() {
  const { session } = useAuth()
  const recentErrors = useRef<Map<string, number>>(new Map())

  useEffect(() => {
    // Only subscribe if authenticated
    if (!session) return

    const sub = graphqlClient
      .subscription<{ errorLogs: LogEventSubscription }>(ERROR_LOGS_SUBSCRIPTION, {})
      .subscribe({
        next: (result) => {
          if (result.data?.errorLogs) {
            const log = result.data.errorLogs

            // Create a key for deduplication
            const key = `${log.target}:${log.message.substring(0, 50)}`

            // Check if we've shown this error recently
            const lastShown = recentErrors.current.get(key)
            const now = Date.now()

            if (lastShown && now - lastShown < DEBOUNCE_WINDOW) {
              // Skip duplicate error
              return
            }

            // Update the timestamp
            recentErrors.current.set(key, now)

            // Clean up old entries
            for (const [k, v] of recentErrors.current.entries()) {
              if (now - v > DEBOUNCE_WINDOW * 2) {
                recentErrors.current.delete(k)
              }
            }

            // Extract the module name from target for a cleaner title
            const targetParts = log.target.split('::')
            const moduleName = targetParts.length > 1 ? targetParts[targetParts.length - 1] : log.target

            // Truncate message if too long
            const message =
              log.message.length > MAX_MESSAGE_LENGTH
                ? `${log.message.substring(0, MAX_MESSAGE_LENGTH)}...`
                : log.message

            addToast({
              title: `Error in ${moduleName}`,
              description: message,
              color: 'danger',
            })
          }
        },
        error: (error) => {
          // Don't spam the user with subscription errors
          console.error('Error log subscription error:', error)
        },
      })

    return () => {
      sub.unsubscribe()
    }
  }, [session])

  // This component doesn't render anything
  return null
}
