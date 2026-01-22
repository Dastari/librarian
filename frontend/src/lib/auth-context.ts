import type { AuthSession, AuthUser } from "./auth";

// Auth context type for router
export interface AuthContext {
  isAuthenticated: boolean;
  isLoading: boolean;
  session: AuthSession | null;
  user: AuthUser | null;
}
