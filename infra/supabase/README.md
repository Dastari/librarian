## Supabase Local Development

There are two ways to run Supabase locally:

### Option 1: Supabase CLI (Recommended for Development)

The predefined and supported way to get the latest Supabase local stack is using the Supabase CLI. It manages Docker images/Compose for you and stays up to date.

#### Prerequisites

- Docker Engine and Docker Compose
- Supabase CLI (install via Homebrew or download a release)

#### Install the Supabase CLI

Homebrew (Linux/macOS):

```bash
brew install supabase/tap/supabase
```

Manual: Download a release from the [Supabase CLI GitHub releases page](https://github.com/supabase/cli/releases).

#### Initialize in this repo (one-time)

```bash
supabase init
```

This creates a `supabase/` directory at the repo root with config managed by the CLI.

#### Start/Stop the local stack

```bash
# Start all Supabase services (Postgres, Auth, Storage, Studio, etc.)
supabase start

# Stop services
supabase stop

# Reset the database (DANGER: drops local data)
supabase db reset
```

After `start`, the CLI prints connection details. Typical local ports:

- Studio: http://localhost:54323
- REST API: http://localhost:54321
- Postgres: localhost:54322

#### Obtain keys and URLs

```bash
supabase status
```

Use the printed API URL, anon key, and service role key for your backend `.env`.

### Option 2: Docker Compose (Self-Hosting)

For production or when you need more control, you can use the official Supabase Docker setup.

#### Prerequisites

- Docker Engine and Docker Compose
- Git

#### Setup

The Supabase Docker files have been cloned to `infra/supabase-docker/`:

```bash
cd infra/supabase-docker/docker

# The .env file has been created from .env.example
# Edit .env to configure:
# - POSTGRES_PASSWORD: Strong database password
# - JWT_SECRET: Secure JWT secret
# - SITE_URL: Your site URL
# - SMTP_*: Email server settings

# Start all services
docker compose up -d

# Stop services
docker compose down
```

#### Default Ports (Docker)

- Studio: http://localhost:3000
- REST API: http://localhost:8000
- Postgres: localhost:5432

#### Generate Secure Keys

For production, generate proper API keys using your JWT secret:

```bash
# Generate anon key
node -e "
const jwt = require('jsonwebtoken');
const token = jwt.sign(
  { role: 'anon', iss: 'supabase', iat: Math.floor(Date.now() / 1000), exp: Math.floor(Date.now() / 1000) + 31536000 },
  process.env.JWT_SECRET || 'your-jwt-secret'
);
console.log(token);
"
```

### Which to Use?

| Scenario | Recommendation |
|----------|----------------|
| Local development | CLI (`supabase start`) |
| CI/CD testing | CLI |
| Production self-hosting | Docker Compose |
| NAS deployment | Docker Compose |

### Environment Variables

After starting Supabase, copy these values to your `.env` file:

```bash
# From `supabase status` or Docker .env
SUPABASE_URL=http://localhost:54321          # or :8000 for Docker
SUPABASE_ANON_KEY=<your-anon-key>
SUPABASE_SERVICE_KEY=<your-service-key>
JWT_SECRET=<your-jwt-secret>
DATABASE_URL=postgresql://postgres:postgres@localhost:54322/postgres
```

### Connecting the Backend

The Librarian backend connects to Supabase via:
- Direct PostgreSQL connection (sqlx)
- Storage API for artwork
- JWT verification for auth

Make sure the database is accessible before starting the backend.
