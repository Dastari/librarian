# Librarian Makefile
# Common commands for development and deployment

.PHONY: help dev dev-backend dev-frontend \
        build docker-up docker-down docker-logs clean test lint distro \
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
	@echo "  make distro         - Build distro artifacts (Linux + Windows)"
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

# =============================================================================
# Database Migrations
# =============================================================================

# Default SQLite database path for local development
DATABASE_PATH ?= ./data/librarian.db
DATABASE_URL ?= sqlite://$(DATABASE_PATH)

db-migrate:
	@echo "Running database migrations..."
	cd backend && DATABASE_URL="$(DATABASE_URL)" sqlx migrate run --source migrations_sqlite

db-migrate-info:
	@echo "Migration status:"
	cd backend && DATABASE_URL="$(DATABASE_URL)" sqlx migrate info --source migrations_sqlite

db-migrate-revert:
	@echo "Reverting last migration..."
	cd backend && DATABASE_URL="$(DATABASE_URL)" sqlx migrate revert --source migrations_sqlite

db-migrate-add:
ifndef NAME
	$(error NAME is required. Usage: make db-migrate-add NAME=my_migration)
endif
	@echo "Creating new migration: $(NAME)"
	cd backend && sqlx migrate add $(NAME) --source migrations_sqlite

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
# Distribution
# =============================================================================

distro:
	./scripts/build-distro.sh

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
