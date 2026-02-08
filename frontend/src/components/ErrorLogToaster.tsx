/**
 * ErrorLogToaster Component
 *
 * Listens for ERROR level log events via GraphQL subscription and displays them as toasts.
 * Should be mounted once in the app root to provide global error notifications.
 */

import { useEffect, useRef } from 'react'
import { addToast } from '@heroui/toast'
import {
  AppLogChangedDocument,
  type AppLogChangedSubscription,
  type AppLogChangedSubscriptionVariables,
} from '../lib/graphql/generated/graphql'
import { subscriptionStream } from '../lib/graphql/client'
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

    const sub = subscriptionStream<
      AppLogChangedSubscription,
      AppLogChangedSubscriptionVariables
    >(AppLogChangedDocument, {}).subscribe({
        next: (result: any) => {
          const log = result.data?.AppLogChanged?.AppLog
          if (log && log.Level === 'ERROR') {

            // Create a key for deduplication
            const key = `${log.Target}:${log.Message.substring(0, 50)}`

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
            const targetParts = log.Target.split('::')
            const moduleName = targetParts.length > 1 ? targetParts[targetParts.length - 1] : log.Target

            // Truncate message if too long
            const message =
              log.Message.length > MAX_MESSAGE_LENGTH
                ? `${log.Message.substring(0, MAX_MESSAGE_LENGTH)}...`
                : log.Message

            addToast({
              title: `Error in ${moduleName}`,
              description: message,
              color: 'danger',
            })
          }
        },
        error: (error: any) => {
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
