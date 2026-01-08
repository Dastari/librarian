# Librarian Makefile
# Common commands for development and deployment

.PHONY: help dev dev-backend dev-frontend supabase-start supabase-stop \
        build docker-up docker-down docker-logs clean test lint

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
	@echo "Docker:"
	@echo "  make docker-up      - Start all Docker services"
	@echo "  make docker-down    - Stop all Docker services"
	@echo "  make docker-logs    - View Docker service logs"
	@echo "  make docker-build   - Build Docker images"
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
# Docker
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
