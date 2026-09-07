import type { AuthContext } from "./auth-context";
import {
  clearTokens,
  getSession,
  hasValidToken,
  setTokens,
  type AuthSession,
} from "./auth";
import { apolloClient } from "./graphql/client";
import { RefreshTokenDocument } from "./graphql/generated/graphql";

/**
 * Ensure we have a valid access token for protected routes.
 * - If access token is valid, returns true.
 * - If access token expired but refresh token is valid, refreshes and returns true.
 * - Otherwise clears auth state and returns false.
 */
export async function ensureAuthenticated(
  auth: AuthContext,
): Promise<boolean> {
  if (auth.isAuthenticated && hasValidToken()) {
    return true;
  }

  const existingSession = getSession();
  if (!existingSession?.user) {
    clearTokens();
    return false;
  }

  try {
    const result = await apolloClient.mutate({
      mutation: RefreshTokenDocument,
    });

    const payload = result.data?.refreshToken;
    if (!payload?.success || !payload.tokens) {
      clearTokens();
      return false;
    }

    const tokens = payload.tokens;
    const newSession: AuthSession = {
      expiresAt: Math.floor(Date.now() / 1000) + tokens.expiresIn,
      user: existingSession.user,
    };
    setTokens(newSession);
    return true;
  } catch {
    clearTokens();
    return false;
  }
}
