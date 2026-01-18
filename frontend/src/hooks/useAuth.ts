import { useState, useEffect, useCallback } from 'react'
import { supabase, isSupabaseConfigured, type User } from '../lib/supabase'
import type { AuthContext } from '../lib/auth-context'
import { DEFAULT_AUTH_STATE } from '../lib/auth-context'

/**
 * Core auth state hook used by both main.tsx (for router context) and useAuth (for components).
 * This centralizes the auth state management logic to avoid duplication.
 */
export function useAuthState(): AuthContext {
  const [auth, setAuth] = useState<AuthContext>(DEFAULT_AUTH_STATE)

  useEffect(() => {
    if (!isSupabaseConfigured) {
      setAuth({ isAuthenticated: false, isLoading: false, session: null })
      return
    }

    // Get initial session
    supabase.auth.getSession().then(({ data: { session } }) => {
      setAuth({
        isAuthenticated: !!session,
        isLoading: false,
        session,
      })
    }).catch(() => {
      setAuth({ isAuthenticated: false, isLoading: false, session: null })
    })

    // Listen for auth changes - skip token refresh events to avoid unnecessary re-renders
    const { data: { subscription } } = supabase.auth.onAuthStateChange((event, session) => {
      // Skip token refresh events - they don't change auth status
      // and would cause unnecessary re-renders (e.g., when alt-tabbing back)
      if (event === 'TOKEN_REFRESHED') {
        return
      }
      
      setAuth((prev) => {
        // Only update if authentication status actually changed
        const isAuthenticated = !!session
        const sessionChanged = prev.session?.access_token !== session?.access_token

        // Skip update if nothing meaningful changed
        if (prev.isAuthenticated === isAuthenticated && !prev.isLoading && !sessionChanged) {
          return prev // Return same reference to avoid re-render
        }

        return {
          isAuthenticated,
          isLoading: false,
          session,
        }
      })
    })

    return () => subscription.unsubscribe()
  }, [])

  return auth
}

/**
 * Full auth hook with sign in/out functionality.
 * Use this in components that need to interact with auth (sign in forms, navbar, etc.)
 */
export function useAuth() {
  const auth = useAuthState()
  const [error, setError] = useState<string | null>(null)
  
  // Derive user from session
  const user: User | null = auth.session?.user ?? null

  const signIn = useCallback(async (email: string, password: string) => {
    setError(null)
    const { error } = await supabase.auth.signInWithPassword({ email, password })
    if (error) {
      setError(error.message)
      throw error
    }
  }, [])

  const signUp = useCallback(async (email: string, password: string) => {
    setError(null)
    const { error } = await supabase.auth.signUp({ email, password })
    if (error) {
      setError(error.message)
      throw error
    }
  }, [])

  const signOut = useCallback(async () => {
    setError(null)
    const { error } = await supabase.auth.signOut()
    if (error) {
      setError(error.message)
      throw error
    }
  }, [])

  return {
    user,
    session: auth.session,
    loading: auth.isLoading,
    error,
    isConfigured: isSupabaseConfigured,
    signIn,
    signUp,
    signOut,
  }
}
