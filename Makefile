# Librarian Makefile
# Common commands for development and deployment

.PHONY: help dev dev-backend dev-frontend supabase-start supabase-stop \
        build docker-up docker-down docker-logs clean test lint \
        db-migrate db-migrate-info db-migrate-revert db-migrate-add

# Default target
help:
	@echo "Librarian Development Commands"
	@echo ""
	@echo "Development:"
	@echo "  make dev            - Start all development services"
	@echo "  make dev-backend    - Start Rust backend in dev mode"
	@echo "  make dev-frontend   - Start frontend in dev mode"
	@echo ""
	@echo "Supabase:"
	@echo "  make supabase-start - Start Supabase local stack"
	@echo "  make supabase-stop  - Stop Supabase local stack"
	@echo "  make supabase-reset - Reset Supabase database"
	@echo "  make supabase-status - Show Supabase status and keys"
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
	@echo "  make test           - Run all tests"
	@echo "  make lint           - Run linters"
	@echo "  make clean          - Clean build artifacts"

# =============================================================================
# Development
# =============================================================================

dev: supabase-start
	@echo "Starting development services..."
	@make -j2 dev-backend dev-frontend

dev-backend:
	cd backend && cargo watch -x run

dev-frontend:
	cd frontend && pnpm run dev

# =============================================================================
# Supabase
# =============================================================================

supabase-start:
	@echo "Starting Supabase local stack..."
	supabase start

supabase-stop:
	@echo "Stopping Supabase local stack..."
	supabase stop

supabase-reset:
	@echo "Resetting Supabase database..."
	supabase db reset

supabase-status:
	supabase status

# =============================================================================
# Database Migrations
# =============================================================================

# Default database URL for local Supabase
DATABASE_URL ?= postgresql://postgres:postgres@127.0.0.1:54322/postgres

db-migrate:
	@echo "Running database migrations..."
	cd backend && DATABASE_URL="$(DATABASE_URL)" sqlx migrate run

db-migrate-info:
	@echo "Migration status:"
	cd backend && DATABASE_URL="$(DATABASE_URL)" sqlx migrate info

db-migrate-revert:
	@echo "Reverting last migration..."
	cd backend && DATABASE_URL="$(DATABASE_URL)" sqlx migrate revert

db-migrate-add:
ifndef NAME
	$(error NAME is required. Usage: make db-migrate-add NAME=my_migration)
endif
	@echo "Creating new migration: $(NAME)"
	cd backend && sqlx migrate add $(NAME)

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

build: build-backend build-frontend

build-backend:
	cd backend && cargo build --release

build-frontend:
	cd frontend && pnpm run build

# =============================================================================
# Testing
# =============================================================================

test: test-backend test-frontend

test-backend:
	cd backend && cargo test

test-frontend:
	cd frontend && pnpm test

# =============================================================================
# Linting
# =============================================================================

lint: lint-backend lint-frontend

lint-backend:
	cd backend && cargo clippy -- -D warnings
	cd backend && cargo fmt --check

lint-frontend:
	cd frontend && pnpm run lint

# =============================================================================
# Clean
# =============================================================================

clean:
	cd backend && cargo clean
	cd frontend && rm -rf node_modules .output dist pnpm-lock.yaml
