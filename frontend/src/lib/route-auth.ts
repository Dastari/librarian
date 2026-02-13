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
      variables: { input: { RefreshToken: "" } },
    });

    const payload = result.data?.RefreshToken;
    if (!payload?.Success || !payload.Tokens) {
      clearTokens();
      return false;
    }

    const tokens = payload.Tokens;
    const newSession: AuthSession = {
      accessToken: tokens.AccessToken,
      expiresAt: Date.now() + tokens.ExpiresIn * 1000,
      user: existingSession.user,
    };
    setTokens(newSession);
    return true;
  } catch {
    clearTokens();
    return false;
  }
}
