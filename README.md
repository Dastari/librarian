## Librarian

Librarian is a local-first, privacy-preserving media library that runs on a single machine or NAS, works offline, and optionally supports remote access. It automates downloading, organizing, and streaming your media with a modern web UI and a robust, Rust-powered backend.

### Goals & Aims

- **Local-first privacy**: Your library, your machine. Cloud-optional.
- **Offline-capable**: Daily usage should not require the internet.
- **End-to-end usability**: Download → organize → browse → play → cast.
- **Automation**: Subscriptions to shows and "fill the gaps" via indexers.
- **Simplicity & reliability**: Prefer stable APIs and clear components.

### Core Features

- **Native torrent downloads** via embedded librqbit (pure Rust BitTorrent client)
- **Real-time updates** via GraphQL subscriptions over WebSocket
- **In-browser streaming** with HLS; direct play when possible
- **Casting**: Chromecast (Google Cast Sender SDK) and AirPlay (native Safari)
- **Playback UI** with search, details pages, and library browsing
- **Metadata fetching** from TheTVDB (series/episodes) and TMDB (movies/art)
- **Subscriptions** for shows, with automated searching via Torznab indexers
- **Auto-rename & organization** of media files into predictable folders
- **Background workers** for scanning, RSS polling, monitoring, transcoding GC

### Architecture Overview

| Component | Technology |
|-----------|------------|
| **Frontend** | TanStack Start, TypeScript, HeroUI + Tailwind CSS, pnpm |
| **Backend** | Rust (Axum + Tokio), `sqlx` (Postgres), job scheduling |
| **API** | GraphQL with subscriptions (async-graphql) |
| **Identity & DB** | Supabase (Auth, Postgres with RLS, Storage) - local Docker |
| **Torrent Engine** | librqbit (native Rust, embedded) with DHT, PEX, magnet support |
| **Indexers** | Prowlarr (recommended) or Jackett as Torznab aggregator |
| **Transcoding** | FFmpeg/FFprobe → HLS playlists/segments |
| **Storage** | Supabase Storage for posters/backdrops/thumbnails |

For a deeper dive into all components and APIs, see the design document:
- [System Design](docs/design.md)
- [Implementation Plan](docs/implementation-plan.md)

### Repository Structure

```
librarian/
├── docs/                      # Design docs and implementation plan
├── infra/
│   ├── supabase/              # Supabase CLI setup notes
│   └── supabase-docker/       # Cloned Supabase Docker setup
├── backend/                   # Rust Axum API service
│   ├── src/
│   │   ├── api/               # REST endpoints (file upload, filesystem only)
│   │   ├── auth/              # JWT verification
│   │   ├── config/            # Configuration
│   │   ├── db/                # Database connection
│   │   ├── graphql/           # GraphQL schema, subscriptions
│   │   ├── jobs/              # Background workers
│   │   ├── media/             # Transcoding, metadata, organizer
│   │   ├── services/          # Service clients (torrent, prowlarr, storage)
│   │   └── torrent/           # Quality profiles
│   └── migrations/            # SQL migrations
├── frontend/                  # TanStack Start web app
│   └── src/
│       ├── components/        # Reusable UI components
│       ├── hooks/             # React hooks
│       ├── lib/               # Supabase client, API client
│       └── routes/            # File-based routes
├── docker-compose.yml         # Application services
├── Makefile                   # Development commands
└── .env.example               # Environment template
```

### Getting Started

#### Prerequisites

- **Docker** and **Docker Compose**
- **Rust toolchain** (1.75+): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Node.js** (20+): Use nvm or download from nodejs.org
- **Supabase CLI**: `brew install supabase/tap/supabase` or download from releases

#### Quick Start

1. **Clone and setup environment**
   ```bash
   cd librarian
   cp .env.example .env
   ```

2. **Start Supabase local stack**
   ```bash
   supabase start
   ```
   Copy the displayed keys into your `.env` file.

3. **Start development servers**
   ```bash
   # Terminal 1: Backend
   cd backend
   cargo run

   # Terminal 2: Frontend
   cd frontend
   pnpm install
   pnpm run dev
   ```

4. **Access the app**
   - Frontend: http://localhost:3000
   - Backend API: http://localhost:3001
   - Supabase Studio: http://localhost:54323

#### Using Docker (Production-like)

```bash
# Start all services (requires Supabase running separately)
make docker-up

# With Prowlarr for indexer management
make docker-up-with-indexers

# View logs
make docker-logs

# Stop services
make docker-down
```

#### Using Make Commands

```bash
make help           # Show all available commands
make dev            # Start full development environment
make dev-backend    # Start only backend
make dev-frontend   # Start only frontend
make supabase-start # Start Supabase
make supabase-status # Show Supabase keys
make build          # Build all projects
make test           # Run all tests
make lint           # Run linters
```

### Configuration

#### Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `SUPABASE_URL` | Supabase API URL | Yes |
| `SUPABASE_ANON_KEY` | Public anon key | Yes |
| `SUPABASE_SERVICE_KEY` | Service role key (backend only) | Yes |
| `JWT_SECRET` | JWT signing secret | Yes |
| `DATABASE_URL` | PostgreSQL connection URL | Yes |
| `DOWNLOADS_PATH` | Directory for torrent downloads | No (default: `/data/downloads`) |
| `SESSION_PATH` | Directory for torrent session/DHT state | No (default: `/data/session`) |
| `TORRENT_ENABLE_DHT` | Enable DHT for peer discovery | No (default: `true`) |
| `TORRENT_LISTEN_PORT` | Port for incoming torrent connections | No (default: random) |
| `TORRENT_MAX_CONCURRENT` | Max concurrent downloads | No (default: `5`) |
| `TVDB_API_KEY` | TheTVDB API key | No |
| `TMDB_API_KEY` | TMDB API key | No |

### API

The backend exposes a single **GraphQL API** for all operations, with WebSocket subscriptions for real-time updates.

#### Endpoints

| Endpoint | Description |
|----------|-------------|
| `/healthz` | Health check (REST) |
| `/readyz` | Readiness check with DB (REST) |
| `/graphql` | GraphQL API (GET for playground, POST for operations) |
| `/graphql/ws` | WebSocket endpoint for GraphQL subscriptions |
| `/api/torrents/upload` | Torrent file upload (REST - multipart form) |
| `/api/filesystem/browse` | Filesystem browser (REST) |
| `/api/filesystem/mkdir` | Create directory (REST) |

#### Authentication

All GraphQL operations (except `health` and `version` queries) require authentication via JWT in the `Authorization` header:

```
Authorization: Bearer <supabase_jwt_token>
```

For WebSocket subscriptions, pass the token in connection parameters:
```json
{ "Authorization": "Bearer <token>" }
```

#### Example Queries

**Get current user:**
```graphql
query {
  me {
    id
    email
    role
  }
}
```

**List libraries:**
```graphql
query {
  libraries {
    id
    name
    libraryType
    itemCount
    lastScannedAt
  }
}
```

**Add a torrent:**
```graphql
mutation {
  addTorrent(input: { magnet: "magnet:?xt=urn:btih:..." }) {
    success
    torrent {
      id
      name
      progress
    }
    error
  }
}
```

#### Example Subscriptions

**Real-time torrent progress:**
```graphql
subscription {
  torrentProgress {
    id
    infoHash
    progress
    downloadSpeed
    uploadSpeed
    peers
    state
  }
}
```

**Torrent completion events:**
```graphql
subscription {
  torrentCompleted {
    id
    name
    infoHash
  }
}
```

#### Full Schema

Visit the GraphQL playground at `http://localhost:3001/graphql` to explore the full schema with documentation.

### Security

- Verify Supabase JWTs via JWKS; cache keys in the backend
- Enforce RLS for user-scoped tables; use service role only in worker/system tasks
- Use short-lived, signed URLs for HLS segments/playlists and artwork
- Keep the Supabase service role key strictly on the backend

### Development

#### Backend (Rust)

```bash
cd backend

# Run with auto-reload
cargo watch -x run

# Run tests
cargo test

# Check for issues
cargo clippy

# Format code
cargo fmt
```

#### Frontend (TanStack Start + HeroUI)

```bash
cd frontend

# Install dependencies
pnpm install

# Start dev server
pnpm run dev

# Build for production
pnpm run build

# Lint
pnpm run lint
```

### Roadmap

We ship vertical slices that exercise the full stack end-to-end:

1. ✅ Environment & scaffolding (Docker Compose, Supabase CLI), health endpoints
2. ✅ Native torrent client (librqbit) with GraphQL subscriptions
3. ⏳ Auth (Supabase ↔ Frontend ↔ Rust), `GET /api/me`
4. ⏳ Libraries & basic scan (index files; simple UI to browse)
5. ⏳ Playback (direct play first), then single-rung HLS fallback
6. ⏳ Metadata fetch/normalize (TheTVDB/TMDB, artwork in Storage)
7. ⏳ Organization & rename (Movies/TV schemes)
8. ⏳ Subscriptions & "fill the gaps" (Prowlarr/Jackett + Torznab)
9. ⏳ Casting (Chromecast, AirPlay) and streaming enhancements
10. ⏳ Admin settings, testing/observability, packaging/deployment

### License

TBD
