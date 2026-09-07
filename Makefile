# Librarian Makefile
# Common commands for development and deployment

.PHONY: help dev dev-backend dev-frontend \
        build docker-up docker-down docker-logs clean test lint distro \
        security security-backend security-frontend \
        build-backend-debug build-backend-release build-windows-debug build-windows-release \
        windows-installer \
        windows-package-only \
        db-migrate db-migrate-info db-migrate-revert db-migrate-add

# Default target
help:
	@echo "Librarian Development Commands"
	@echo ""
	@echo "Development:"
	@echo "  make dev            - Start all development services"
	@echo "  make dev-backend    - Start Rust backend in dev mode"
	@echo "  make dev-frontend   - Start frontend in dev mode"
	@echo "  make dev-web        - Start the new web frontend (web/) on :3000"
	@echo "  make live-web       - (Re)start the live web frontend on :3000 in the background (web/live.sh)"
	@echo "  make build-web      - Build the new web frontend"
	@echo ""
	@echo "Database:"
	@echo "  make db-migrate     - Run all pending migrations"
	@echo "  make db-migrate-info - Show migration status"
	@echo "  make db-migrate-revert - Revert the last migration"
	@echo "  make db-migrate-add NAME=<name> - Create a new migration"
	@echo ""
	@echo "Docker (Development):"
	@echo "  make docker-up      - Start all Docker services"
	@echo "  make docker-down    - Stop all Docker services"
	@echo "  make docker-logs    - View Docker service logs"
	@echo "  make docker-build   - Build Docker images"
	@echo ""
	@echo "Docker (Production):"
	@echo "  make prod-build     - Build production Docker images"
	@echo "  make prod-up        - Start production services"
	@echo "  make prod-down      - Stop production services"
	@echo "  make prod-logs      - View production logs"
	@echo "  make prod-restart   - Restart production services"
	@echo "  make prod-status    - Show production service status"
	@echo ""
	@echo "Other:"
	@echo "  make build          - Build all projects"
	@echo "  make build-backend-debug  - Build backend debug (no embedded frontend)"
	@echo "  make build-backend-release - Build backend release (embedded frontend)"
	@echo "  make build-windows-debug  - Build Windows debug exe from WSL (no embedded frontend)"
	@echo "  make build-windows-release - Build Windows release exe from WSL (embedded frontend)"
	@echo "  make test           - Run all tests"
	@echo "  make lint           - Run linters"
	@echo "  make distro         - Build distro artifacts (Linux + Windows)"
	@echo "  make windows-installer - Build Windows installer artifacts only"
	@echo "  make windows-package-only - Package MSI/EXE from an existing Windows exe"
	@echo "  make clean          - Clean build artifacts"

# =============================================================================
# Development
# =============================================================================

dev:
	@echo "Starting development services..."
	@make -j2 dev-backend dev-frontend

dev-backend:
	cd backend && cargo watch -x run

dev-frontend:
	cd frontend && pnpm run dev

dev-web:
	cd web && pnpm run dev

live-web:
	cd web && ./live.sh restart

build-web:
	cd web && pnpm install && pnpm run build

lint-web:
	cd web && pnpm run lint

test-web:
	cd web && pnpm test

# =============================================================================
# Database Schema
# =============================================================================

# Default SQLite database path for local development
DATABASE_PATH ?= ./data/librarian.db
DATABASE_URL ?= sqlite://$(DATABASE_PATH)

db-migrate:
	@echo "Database schema is managed by graphql-orm on backend startup."

db-migrate-info:
	@echo "No SQL migration status is available; graphql-orm owns schema sync."

db-migrate-revert:
	@echo "SQL migration revert is not supported; reset the development database file instead."

db-migrate-add:
	@echo "Do not add SQL migrations. Update graphql-orm entities and bootstrap defaults instead."

# =============================================================================
# Docker (Development)
# =============================================================================

docker-up:
	docker compose up -d

docker-down:
	docker compose down

docker-logs:
	docker compose logs -f

docker-build:
	docker compose build

docker-up-with-indexers:
	docker compose --profile indexers up -d

docker-restart:
	docker compose down && docker compose up -d

# =============================================================================
# Docker (Production)
# =============================================================================

prod-build:
	@echo "Building production images..."
	docker compose -f docker-compose.prod.yml build

prod-up:
	@echo "Starting production services..."
	docker compose -f docker-compose.prod.yml up -d

prod-down:
	@echo "Stopping production services..."
	docker compose -f docker-compose.prod.yml down

prod-logs:
	docker compose -f docker-compose.prod.yml logs -f

prod-restart:
	docker compose -f docker-compose.prod.yml down && docker compose -f docker-compose.prod.yml up -d

prod-status:
	docker compose -f docker-compose.prod.yml ps

prod-pull:
	docker compose -f docker-compose.prod.yml pull

# =============================================================================
# Build
# =============================================================================

build: build-backend-release build-frontend

build-backend: build-backend-release

build-backend-debug:
	cd backend && cargo build --locked

build-backend-release:
	cd backend && cargo build --locked --release --features embed-frontend

build-frontend:
	cd frontend && pnpm install && pnpm run build

# =============================================================================
# Windows (from WSL)
# =============================================================================

WIN_BACKEND_DIR_RAW := $(shell wslpath -w $(PWD)/backend)
WIN_BACKEND_DIR := $(subst \,\\,$(WIN_BACKEND_DIR_RAW))

build-windows-debug:
	powershell.exe -NoProfile -Command "Set-Location '$(WIN_BACKEND_DIR)'; [Environment]::SetEnvironmentVariable('CARGO_INCREMENTAL','0','Process'); cargo build --locked --target x86_64-pc-windows-msvc"

build-windows-release: build-frontend
	powershell.exe -NoProfile -Command "Set-Location '$(WIN_BACKEND_DIR)'; [Environment]::SetEnvironmentVariable('CARGO_INCREMENTAL','0','Process'); cargo build --locked --release --features embed-frontend --target x86_64-pc-windows-msvc"

# =============================================================================
# Distribution
# =============================================================================

distro:
	bash ./scripts/build-distro.sh

windows-installer:
	bash ./scripts/build-distro.sh --windows-only

windows-package-only:
	bash ./scripts/build-distro.sh --windows-package-only

# =============================================================================
# Testing
# =============================================================================

test: test-backend test-frontend

test-backend:
	cd backend && cargo test --locked

test-frontend:
	cd frontend && pnpm test

# =============================================================================
# Linting
# =============================================================================

lint: lint-backend lint-frontend

lint-backend:
	cd backend && cargo clippy --locked --all-targets -- -D warnings
	cd backend && cargo fmt --check

lint-frontend:
	cd frontend && pnpm run lint

security: security-backend security-frontend

security-backend:
	cd backend && cargo audit
	cd backend && cargo deny check
	cd backend && cargo machete

security-frontend:
	cd frontend && pnpm run audit:ci

# =============================================================================
# Clean
# =============================================================================

clean:
	cd backend && cargo clean
	cd frontend && rm -rf node_modules .output dist pnpm-lock.yaml
