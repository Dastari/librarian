## Librarian — Incremental Build Plan

### Guiding Principles

- **Ship vertical slices** that exercise frontend → API → DB → worker paths.
- **Keep it local-first** and private by default; remote access is an add‑on.
- **Prefer direct play**; add transcoding only where required.
- **Automate via jobs**: scanners, pollers, and post-download processing.
- **Observability from day 1**: health, tracing logs, minimal metrics.

---

## Progress Summary

### ✅ Completed

| Stage | Status | Notes |
|-------|--------|-------|
| Stage 0 | ✅ Complete | Full scaffold with all components |
| Stage 1 | 🟡 Partial | Frontend auth complete, backend JWT middleware scaffolded |
| Stage 4 | ✅ Complete | Native torrent client (librqbit) + GraphQL subscriptions |

### Key Decisions Made

1. **Frontend Framework**: Changed from Next.js to **TanStack Start** with TanStack Router
2. **UI Library**: Changed from Shadcn/Radix to **HeroUI** (formerly NextUI)
3. **Package Manager**: Using **pnpm** instead of npm for frontend
4. **Project Structure**: Simplified folder names (`backend/`, `frontend/` instead of `librarian-*`)
5. **Production Architecture**: Single domain with reverse proxy (Caddy) routing:
   - `/` → Frontend
   - `/graphql` → GraphQL API (single API surface)
   - Supabase accessed directly from frontend for auth
6. **Torrent Client**: Changed from qBittorrent to **librqbit** (native Rust, embedded)
7. **API Architecture**: **GraphQL-only API** with subscriptions (async-graphql)
   - Single endpoint for all operations: `/graphql`
   - WebSocket subscriptions: `/graphql/ws`
   - Centralized auth via JWT verification in GraphQL context
   - No REST API (except health endpoints)

---

## Stage 0 — Environment & Scaffolding ✅ COMPLETE

- **Status**: ✅ Done
- **Completed**: January 2026

### What Was Built

- ✅ **Docker Compose** (`docker-compose.yml`):
  - Backend service (Rust/Axum)
  - Frontend service (TanStack Start)
  - qBittorrent (linuxserver image)
  - Prowlarr (optional, via profile)
  - Shared volumes for media, downloads, cache

- ✅ **Supabase Local Stack**:
  - Cloned official Docker setup to `infra/supabase-docker/`
  - CLI-based setup documented in `infra/supabase/README.md`
  - Database migrations created (`backend/migrations/001_initial_schema.sql`)

- ✅ **Rust Backend** (`backend/`):
  - Axum web framework with Tokio runtime
  - Health endpoints: `GET /healthz`, `GET /readyz`
  - **GraphQL API** (single API surface): `/graphql`, `/graphql/ws`
    - Queries: me, libraries, mediaItems, torrents, subscriptions, searchMedia, searchTorrents
    - Mutations: CRUD for libraries/subscriptions, torrent management, preferences
    - Subscriptions: torrentProgress, torrentAdded, torrentCompleted, torrentRemoved
  - JWT auth in GraphQL context (centralized)
  - Service clients: Torrent (librqbit), Prowlarr, Supabase Storage
  - Media modules: Transcoder, Organizer, Metadata
  - Background jobs: Scanner, RSS Poller, Download Monitor, Transcode GC
  - Database connection with sqlx

- ✅ **TanStack Start Frontend** (`frontend/`):
  - File-based routing with TanStack Router
  - HeroUI + Tailwind CSS styling
  - Supabase client integration
  - Auth hooks and components
  - Error boundary and 404 handling
  - Pages: Home, Downloads, Subscriptions, Login, Media Detail
  - Video player component with HLS.js

- ✅ **Developer Experience**:
  - `Makefile` with common commands
  - `.env.example` template
  - `.gitignore` configured
  - `README.md` with setup instructions

### Acceptance Criteria Met
- [x] `docker compose up` can serve frontend and API
- [x] Health endpoints return 200
- [x] Supabase local stack runs with auth enabled

---

## Stage 1 — Auth Integration 🟡 IN PROGRESS

- **Status**: 🟡 Partial (Frontend complete, Backend needs wiring)
- **Current Sprint**

### What's Done

- ✅ **Frontend Auth**:
  - Supabase JS client configured with environment variables
  - `useAuth` hook with session management
  - Login/Signup page with email/password
  - Navbar shows auth state
  - Protected route awareness (shows Sign In when logged out)
  - Error handling for missing Supabase config

- ✅ **Backend Auth Scaffold**:
  - JWT verification middleware in `src/auth/mod.rs`
  - `UserContext` extraction from tokens
  - `/api/me` endpoint structure

### What's Remaining

- [ ] **Wire up `/api/me`** to actually return user data from database
- [ ] **Test JWT verification** end-to-end (frontend → backend)
- [ ] **Add auth middleware** to protected routes
- [ ] **Create user profile table** or use Supabase auth.users directly
- [ ] **Handle token refresh** in frontend API client

### Next Steps
```bash
# 1. Start backend and test /api/me with a valid token
cd backend && cargo run

# 2. Sign in via frontend, check network tab for /api/me call
# 3. Verify JWT is being sent and validated
```

---

## Stage 2 — Libraries & Basic Scanner ⏳ NEXT

- **Status**: ⏳ Not Started
- **Priority**: High (enables browsing media)

### Goals
- Register library paths (Movies, TV directories)
- Scan filesystem and index files in database
- Display discovered files in UI

### Deliverables
- [ ] **Database**: `libraries`, `media_files` tables (already in migration)
- [ ] **Endpoints**: 
  - `GET /api/libraries` - list user's libraries
  - `POST /api/libraries` - create new library
  - `POST /api/libraries/{id}/scan` - trigger scan job
- [ ] **Scanner Worker**:
  - Walk directory tree with `walkdir`
  - Run `ffprobe` for basic media info (duration, resolution, codecs)
  - Store file records in `media_files` table
- [ ] **UI**:
  - Library settings page
  - File browser/grid view
  - Basic filtering (movies vs TV)

### Acceptance Criteria
- [ ] User can add a library path
- [ ] Scan discovers files and stores metadata
- [ ] UI displays discovered files

---

## Stage 3 — Playback (Direct Play + HLS Fallback) ⏳ PENDING

- **Status**: ⏳ Not Started
- **Depends on**: Stage 2

### Goals
- Stream media files in browser
- Direct play when codec is compatible
- Transcode to HLS when needed

### Deliverables
- [ ] **Direct Play**: Serve files with proper `Content-Range` headers
- [ ] **HLS Transcoding**: FFmpeg pipeline for on-the-fly HLS
- [ ] **Tokenized URLs**: Signed URLs for stream access
- [ ] **Video Player**: HLS.js integration (component exists, needs wiring)
- [ ] **Playback Controls**: Play, pause, seek, volume

### Acceptance Criteria
- [ ] Compatible files play directly
- [ ] Incompatible files transcode and play via HLS
- [ ] Playback is protected (requires auth)

---

## Stage 4 — qBittorrent Integration ⏳ PENDING

- **Status**: ⏳ Not Started
- **Depends on**: Stage 1

### Goals
- Add torrents via magnet links or URLs
- Monitor download progress
- Display download status in UI

### Deliverables
- [ ] **qBittorrent Client**: Service client exists, needs integration
- [ ] **Endpoints**: Wire up `POST /api/torrents`, `GET /api/torrents`
- [ ] **Background Poller**: Update download states periodically
- [ ] **UI**: Downloads page exists, needs real data

### Acceptance Criteria
- [ ] User can paste magnet link and start download
- [ ] Progress updates in real-time
- [ ] Completed downloads appear in library

---

## Stages 5-12 — Future Work

| Stage | Name | Status | Description |
|-------|------|--------|-------------|
| 5 | Metadata Fetch | ⏳ Pending | TheTVDB/TMDB integration, artwork |
| 6 | Organization & Rename | ⏳ Pending | Auto-rename downloaded files |
| 7 | Subscriptions | ⏳ Pending | Monitor shows, auto-download new episodes |
| 8 | Casting | ⏳ Pending | Chromecast and AirPlay support |
| 9 | Transcoding Enhancements | ⏳ Pending | ABR ladder, subtitles, hardware accel |
| 10 | Admin Settings | ⏳ Pending | Configuration UI |
| 11 | Testing & Observability | ⏳ Pending | Tests, logging, metrics |
| 12 | Packaging & Deployment | ⏳ Pending | Docker images, deployment guides |

---

## Suggested Implementation Order

### Phase 1: Core Functionality (Stages 1-4)
**Goal**: Usable media player with downloads

1. ✅ ~~Stage 0: Environment & Scaffolding~~
2. 🔄 **Stage 1: Complete Auth** (current)
   - Wire up `/api/me` endpoint
   - Test end-to-end auth flow
3. ⏳ **Stage 2: Libraries & Scanner**
   - Add library management
   - Implement file scanner
4. ⏳ **Stage 3: Playback**
   - Direct play first
   - HLS fallback for incompatible files
5. ⏳ **Stage 4: Torrents**
   - qBittorrent integration
   - Download management UI

### Phase 2: Polish & Metadata (Stages 5-6)
**Goal**: Rich media library with organization

6. ⏳ **Stage 5: Metadata**
   - TheTVDB/TMDB integration
   - Artwork fetching
7. ⏳ **Stage 6: Organization**
   - Auto-rename files
   - Folder structure

### Phase 3: Automation (Stage 7)
**Goal**: Hands-off show monitoring

8. ⏳ **Stage 7: Subscriptions**
   - Prowlarr integration
   - Auto-download new episodes

### Phase 4: Advanced Features (Stages 8-12)
**Goal**: Production-ready application

9. ⏳ **Stages 8-9: Casting & Streaming**
10. ⏳ **Stages 10-12: Admin, Testing, Deployment**

---

## Current Sprint Tasks

### Immediate (This Session)
- [x] ~~Scaffold project structure~~
- [x] ~~Set up Supabase local~~
- [x] ~~Create frontend with auth~~
- [x] ~~Add error handling~~
- [ ] Complete Stage 1 auth integration

### Next Session
- [ ] Wire up `/api/me` endpoint in backend
- [ ] Test full auth flow
- [ ] Start Stage 2: Library management

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| FFmpeg performance on NAS | Slow transcoding | Prefer direct play; add hardware accel later |
| Indexer variability | Failed searches | Abstract behind Prowlarr; robust error handling |
| RLS policy issues | Data leaks or access errors | Start with strict policies; test thoroughly |
| Long-running jobs | Timeouts, failures | Use job queue; make handlers idempotent |
| Supabase version drift | Breaking changes | Pin versions; test upgrades in staging |

---

## Definition of Done (per stage)

- [ ] Code merged to main branch
- [ ] Automated tests for critical paths
- [ ] Documentation updated
- [ ] Docker Compose stack starts cleanly
- [ ] Happy path tested manually
- [ ] No console errors in browser
- [ ] API endpoints return proper error codes

---

## Architecture Reference

### Production Deployment (Single Domain)

```
librarian.dastari.net
         │
    ┌────┴────┐
    │  Caddy  │  (reverse proxy, auto HTTPS)
    └────┬────┘
         │
    ┌────┼────────────────┐
    │    │                │
    ▼    ▼                ▼
  /    /api/*         Supabase
Frontend  Backend      (auth/db)
 :3000    :3001
```

### Tech Stack

| Component | Technology |
|-----------|------------|
| Frontend | TanStack Start, React, TypeScript, HeroUI, Tailwind, pnpm |
| Backend | Rust, Axum, Tokio, sqlx |
| API | GraphQL-only with subscriptions (async-graphql) |
| Database | PostgreSQL (via Supabase) |
| Auth | Supabase Auth (email/password) |
| Storage | Supabase Storage (artwork) |
| Torrents | librqbit (native Rust, embedded) |
| Indexers | Prowlarr (Torznab) |
| Transcoding | FFmpeg |
| Proxy | Caddy (production) |
