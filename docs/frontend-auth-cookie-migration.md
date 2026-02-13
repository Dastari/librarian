# Frontend Auth Cookie Migration

Date: 2026-02-13

## Current State

- Frontend access token is currently written and read in `frontend/src/lib/auth.ts` via `document.cookie`.
- Refresh token rotation now uses backend-set `HttpOnly` cookie headers on auth mutations.
- Frontend refresh/logout flows send an empty `RefreshToken` input and rely on backend cookie fallback.
- Token value logging has been removed from frontend auth utilities.

## Risk

- Access token remains JavaScript-readable in the current transitional model.
- Any XSS in the frontend can read access token cookies until access auth is fully server-managed.

## Target State

- Auth cookies are set and rotated by backend responses only.
- Auth cookies are `HttpOnly`, `Secure`, and `SameSite` policy is explicitly configured by backend.
- Frontend no longer reads raw access/refresh token cookie values.
- Frontend auth flow relies on server session state (GraphQL/API calls with browser-managed cookies).

## Migration Outline

1. Add backend login/refresh/logout responses that set/clear `HttpOnly` cookies.
2. Update frontend auth utilities to stop persisting tokens in `document.cookie`.
3. Remove token-bearing GraphQL refresh inputs that require frontend-held refresh token strings.
4. Verify route auth, websocket auth, and cross-tab auth-change behavior with server-managed cookies.
