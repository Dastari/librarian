# Authentication deployment security

Browser authentication uses rotating access and refresh credentials stored only in server-set
`HttpOnly` cookies. GraphQL responses, browser JavaScript, WebSocket connection parameters, media
URLs, and client logs must not contain either credential value.

The access cookie is scoped to `/`. The refresh cookie is scoped to `/graphql`. Both use
`SameSite=Lax`; state-changing cookie-authenticated GraphQL requests additionally require an
`Origin` matching the request origin or an entry in `LIBRARIAN_CORS_ORIGINS`.

## Session lifetime and renewal

Access credentials last 15 minutes. Refresh credentials last 30 days and rotate on every
successful renewal, starting another 30-day window. A browser can therefore return after up
to 30 days without signing in, and continued use keeps the session alive. Logging out,
revoking a session, disabling its account, or replaying a rotated token ends that session.

The browser retains non-secret user/expiry metadata for the same 30 days. It renews early
throughout playback, before protected requests, and on focus, visibility, or network recovery.
Tabs serialize rotation using Web Locks; callers within a tab share one request. Renewal
preserves Apollo's cached content and reconnects subscriptions with the new cookies.
A network or database outage leaves the refresh cookie intact for retry; only confirmed
invalid, expired, or revoked sessions are cleared. Missing display metadata is rebuilt from
the server cookies on startup.

The auth store accepts both the ORM's Unix-second timestamps and legacy RFC3339 timestamps.
No database migration or credential reset is needed for existing valid refresh records.

## HTTPS and reverse proxies

Set `LIBRARIAN_SECURE_COOKIES=true` whenever the public application URL is HTTPS. This is the
preferred configuration and does not depend on proxy-supplied headers.

For the live `https://librarian.dastari.net` frontend, set these in `backend/.env` (the backend's
working directory), then restart the backend:

```dotenv
LIBRARIAN_SECURE_COOKIES=true
LIBRARIAN_CORS_ORIGINS=https://librarian.dastari.net,http://localhost:3000,http://127.0.0.1:3000,http://localhost:3002,http://127.0.0.1:3002
```

`LIBRARIAN_SECURE_COOKIES=true` still emits `Secure` cookies for HTTPS origins such as
`https://librarian.dastari.net`. HTTP Vite origins (`http://localhost:3000` and
`http://localhost:3002`) keep non-Secure cookies so the browser will store them. The cookie
origin guard also treats those HTTP origins as same-origin when the `Host` header matches,
so a mixed public-HTTPS plus local-HTTP setup can sign in on both UIs.

Include the public origin explicitly when a reverse proxy can rewrite the upstream `Host` header.
Verify `NeedsSetup` and WebSocket upgrades with an existing or expired auth cookie: cookie-free
health probes do not exercise the origin guard. Do not disable that guard to fix a deployment.

If cookie security must be derived from a terminating reverse proxy, set
`LIBRARIAN_TRUSTED_PROXIES` to a comma-separated list of the exact direct proxy IP addresses or
CIDR networks. Only requests whose TCP peer belongs to that list may communicate
`proto=https` through `Forwarded` or `X-Forwarded-Proto`. Forwarded headers from every other peer
are ignored.

Example:

```dotenv
LIBRARIAN_SECURE_COOKIES=false
LIBRARIAN_TRUSTED_PROXIES=10.20.0.4/32,fd00:1234::4/128
LIBRARIAN_CORS_ORIGINS=https://library.example
```

Do not use broad trusted-proxy ranges unless the entire range is controlled and cannot be reached
by application clients. Cross-site browser deployments are intentionally unsupported; add an
explicit CSRF token protocol before using `SameSite=None`.

Development on `http://localhost:3000` and `http://localhost:3002` remains supported with
insecure cookies and the default localhost CORS origins. A non-development server logs a
security warning when neither secure cookies nor trusted proxies are configured.
