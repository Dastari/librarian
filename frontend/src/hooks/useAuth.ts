import { useState, useEffect } from 'react'
import { supabase, isSupabaseConfigured, type User, type Session } from '../lib/supabase'

export function useAuth() {
  const [user, setUser] = useState<User | null>(null)
  const [session, setSession] = useState<Session | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!isSupabaseConfigured) {
      setError('Supabase is not configured. Check your environment variables.')
      setLoading(false)
      return
    }

    // Get initial session
    supabase.auth.getSession().then(({ data: { session } }) => {
      setSession(session)
      setUser(session?.user ?? null)
      setLoading(false)
    }).catch((err) => {
      setError(err.message)
      setLoading(false)
    })

    // Listen for auth changes - skip token refresh events to avoid unnecessary re-renders
    const { data: { subscription } } = supabase.auth.onAuthStateChange((event, session) => {
      // Skip token refresh events - they don't change auth status
      // and would cause unnecessary re-renders (e.g., when alt-tabbing back)
      if (event === 'TOKEN_REFRESHED') {
        return
      }
      
      setSession((prev) => {
        // Only update if session actually changed
        if (prev?.access_token === session?.access_token) {
          return prev
        }
        return session
      })
      setUser((prev) => {
        const newUser = session?.user ?? null
        // Only update if user actually changed
        if (prev?.id === newUser?.id) {
          return prev
        }
        return newUser
      })
      setLoading(false)
    })

    return () => subscription.unsubscribe()
  }, [])

  const signIn = async (email: string, password: string) => {
    const { error } = await supabase.auth.signInWithPassword({ email, password })
    if (error) throw error
  }

  const signUp = async (email: string, password: string) => {
    const { error } = await supabase.auth.signUp({ email, password })
    if (error) throw error
  }

  const signOut = async () => {
    const { error } = await supabase.auth.signOut()
    if (error) throw error
  }

  return {
    user,
    session,
    loading,
    error,
    isConfigured: isSupabaseConfigured,
    signIn,
    signUp,
    signOut,
  }
}
