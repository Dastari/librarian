import type { Session } from './supabase'

// Auth context type for router
export interface AuthContext {
  isAuthenticated: boolean
  isLoading: boolean
  session: Session | null
}

// Default auth state
export const DEFAULT_AUTH_STATE: AuthContext = {
  isAuthenticated: false,
  isLoading: true,
  session: null,
}
