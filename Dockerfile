# Librarian - Unified Build
# 
# This Dockerfile builds both frontend and backend into a single image.
# The result is a self-contained application with:
# - Rust backend serving GraphQL API
# - Frontend SPA served from /static
# - SQLite database (no external DB required)
#
# Build: docker build -t librarian .
# Run: docker run -p 3000:3000 -v ./data:/data librarian

# ============================================================================
# Stage 1: Build Frontend
# ============================================================================
FROM node:22-alpine AS frontend-builder

WORKDIR /frontend

# Install pnpm
RUN corepack enable && corepack prepare pnpm@latest --activate

# Copy package files
COPY frontend/package.json frontend/pnpm-lock.yaml frontend/pnpm-workspace.yaml ./

# Install dependencies
RUN pnpm install --frozen-lockfile

# Copy source code
COPY frontend/ ./

# Build arguments for environment variables
# These are baked into the build - use empty string for relative URLs
ARG VITE_API_URL=""

# Build the application
RUN pnpm run build

# ============================================================================
# Stage 2: Build Backend
# ============================================================================
FROM rust:1.83 AS backend-builder

WORKDIR /app

# Install dependencies for sqlx offline mode
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY backend/Cargo.toml backend/Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (cached layer) - using sqlite feature
RUN cargo build --release --features sqlite --no-default-features && rm -rf src

# Copy source code
COPY backend/src ./src
COPY backend/migrations_sqlite ./migrations_sqlite

# Build the application with sqlite feature
RUN touch src/main.rs && cargo build --release --features sqlite --no-default-features

# ============================================================================
# Stage 3: Runtime Image
# ============================================================================
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    ffmpeg \
    curl \
    unrar-free \
    unzip \
    p7zip-full \
    && rm -rf /var/lib/apt/lists/*

# Copy the backend binary
COPY --from=backend-builder /app/target/release/librarian /app/librarian

# Copy SQLite migrations
COPY --from=backend-builder /app/migrations_sqlite /app/migrations_sqlite

# Copy frontend static files
COPY --from=frontend-builder /frontend/dist /app/static

# Create data directories
RUN mkdir -p /data/media /data/downloads /data/cache /data/session

# Default environment variables
ENV PORT=3000
ENV RUST_LOG=info
ENV DATABASE_PATH=/data/librarian.db
ENV MEDIA_PATH=/data/media
ENV DOWNLOADS_PATH=/data/downloads
ENV CACHE_PATH=/data/cache
ENV SESSION_PATH=/data/session

# Expose ports
EXPOSE 3000
EXPOSE 6881
EXPOSE 6881/udp

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:3000/healthz || exit 1

# Run the binary
CMD ["/app/librarian"]
