# Librarian Frontend

React frontend for Librarian using TanStack Router and HeroUI.

## Getting Started

```bash
pnpm install
pnpm dev
```

## Building For Production

```bash
pnpm build
```

## Type Checking

```bash
pnpm exec tsc --noEmit
```

## Tech Stack

- **Framework**: TanStack Router (file-based routing)
- **UI**: HeroUI + Tailwind CSS v4
- **GraphQL**: urql + graphql-ws for subscriptions
- **Auth**: Local JWT auth

## Project Structure

```
src/
├── components/     # Reusable UI components
├── hooks/          # Custom React hooks
├── lib/            # Utilities and clients
│   └── graphql/    # GraphQL queries, mutations, subscriptions, types
└── routes/         # File-based routes (TanStack Router)
```

## Routing

Routes are managed as files in `src/routes/`. TanStack Router automatically generates the route tree.

### Adding A Route

Add a new file in `./src/routes` directory. TanStack will automatically generate the route configuration.

### Using Links

```tsx
import { Link } from "@tanstack/react-router";

<Link to="/libraries">Libraries</Link>
```

## GraphQL

All API operations use GraphQL defined in `src/lib/graphql/`:

- `queries.ts` - Read operations
- `mutations.ts` - Write operations  
- `subscriptions.ts` - Real-time updates
- `types.ts` - TypeScript types

Example usage:

```tsx
import { graphqlClient, LIBRARIES_QUERY, type Library } from '../lib/graphql';

const { data } = await graphqlClient
  .query<{ libraries: Library[] }>(LIBRARIES_QUERY)
  .toPromise();
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `VITE_API_URL` | Optional separate backend URL. Empty/unset uses the page's origin for REST, GraphQL, media, and WebSockets. |
| `BACKEND_PROXY_TARGET` | Vite server-side backend target (default: `http://127.0.0.1:3001`); never exposed as the browser's API address. |
| `DEV_SERVER_PUBLIC_URL` | Public frontend origin when Vite runs behind a reverse proxy; sets its browser WebSocket host, protocol, and port. |

For `https://librarian.dastari.net`, leave `VITE_API_URL` empty or set it to that public origin.
The reverse proxy must forward `/api`, `/graphql`, and `/graphql/ws` to the backend, including
WebSocket upgrades. Vite also proxies these paths for local development.
When serving Vite through the live domain, set `DEV_SERVER_PUBLIC_URL=https://librarian.dastari.net`
and forward Vite's WebSocket upgrades through the frontend proxy too.
