import type { Session } from './supabase'

// Auth context type for router
export interface AuthContext {
  isAuthenticated: boolean
  isLoading: boolean
  session: Session | null
}
