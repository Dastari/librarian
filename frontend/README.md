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
- **Auth**: Supabase Auth

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
| `VITE_API_URL` | Backend API URL (default: `http://localhost:3001`) |
| `VITE_SUPABASE_URL` | Supabase URL |
| `VITE_SUPABASE_ANON_KEY` | Supabase anonymous key |
