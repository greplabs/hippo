.PHONY: help build up down logs restart clean backup restore dev test health stats

# Default target
.DEFAULT_GOAL := help

# Color output
BLUE := \033[0;34m
GREEN := \033[0;32m
YELLOW := \033[0;33m
RED := \033[0;31m
NC := \033[0m # No Color

help: ## Show this help message
	@echo "$(BLUE)Hippo Docker Commands$(NC)"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-15s$(NC) %s\n", $$1, $$2}'
	@echo ""

# Docker Compose commands
build: ## Build Docker images
	@echo "$(BLUE)Building Docker images...$(NC)"
	docker-compose build

up: ## Start services in daemon mode
	@echo "$(BLUE)Starting Hippo services...$(NC)"
	docker-compose up -d
	@echo "$(GREEN)Services started!$(NC)"
	@echo "Web interface: http://localhost:3000"
	@echo "Qdrant dashboard: http://localhost:6333/dashboard"

down: ## Stop and remove services
	@echo "$(YELLOW)Stopping Hippo services...$(NC)"
	docker-compose down
	@echo "$(GREEN)Services stopped$(NC)"

logs: ## View logs (follow mode)
	docker-compose logs -f

logs-web: ## View hippo-web logs only
	docker-compose logs -f hippo-web

logs-qdrant: ## View qdrant logs only
	docker-compose logs -f qdrant

restart: ## Restart all services
	@echo "$(BLUE)Restarting services...$(NC)"
	docker-compose restart
	@echo "$(GREEN)Services restarted$(NC)"

restart-web: ## Restart hippo-web only
	@echo "$(BLUE)Restarting hippo-web...$(NC)"
	docker-compose restart hippo-web
	@echo "$(GREEN)hippo-web restarted$(NC)"

ps: ## Show running containers
	docker-compose ps

stats: ## Show container resource usage
	docker stats hippo-web hippo-qdrant

# Development commands
dev: ## Start in development mode with hot reload
	@echo "$(BLUE)Starting development environment...$(NC)"
	docker-compose -f docker-compose.yml -f docker-compose.dev.yml up
	@echo "$(GREEN)Development environment started$(NC)"

dev-build: ## Build and start development environment
	@echo "$(BLUE)Building development environment...$(NC)"
	docker-compose -f docker-compose.yml -f docker-compose.dev.yml up --build

dev-down: ## Stop development environment
	docker-compose -f docker-compose.yml -f docker-compose.dev.yml down

# Health and monitoring
health: ## Check service health
	@echo "$(BLUE)Checking Hippo web health...$(NC)"
	@curl -f http://localhost:3000/health && echo "$(GREEN)✓ Hippo web is healthy$(NC)" || echo "$(RED)✗ Hippo web is down$(NC)"
	@echo ""
	@echo "$(BLUE)Checking Qdrant health...$(NC)"
	@curl -sf http://localhost:6333/ > /dev/null && echo "$(GREEN)✓ Qdrant is healthy$(NC)" || echo "$(RED)✗ Qdrant is down$(NC)"

test-api: ## Test API endpoints
	@echo "$(BLUE)Testing health endpoint...$(NC)"
	curl -s http://localhost:3000/health | jq .
	@echo ""
	@echo "$(BLUE)Testing stats endpoint...$(NC)"
	curl -s http://localhost:3000/api/stats | jq .
	@echo ""
	@echo "$(BLUE)Testing sources endpoint...$(NC)"
	curl -s http://localhost:3000/api/sources | jq .

# Data management
backup: ## Backup database and Qdrant data
	@echo "$(BLUE)Creating backups...$(NC)"
	@mkdir -p backups
	docker run --rm -v hippo-data:/data -v $$(pwd)/backups:/backup alpine tar czf /backup/hippo-data-$$(date +%Y%m%d-%H%M%S).tar.gz /data
	docker run --rm -v qdrant-data:/qdrant -v $$(pwd)/backups:/backup alpine tar czf /backup/qdrant-data-$$(date +%Y%m%d-%H%M%S).tar.gz /qdrant
	@echo "$(GREEN)Backups created in ./backups/$(NC)"
	@ls -lh backups/ | tail -2

restore: ## Restore from most recent backup (use BACKUP_FILE=filename to specify)
	@if [ -z "$(BACKUP_FILE)" ]; then \
		echo "$(RED)Error: Please specify BACKUP_FILE=filename$(NC)"; \
		echo "Example: make restore BACKUP_FILE=backups/hippo-data-20250101-120000.tar.gz"; \
		exit 1; \
	fi
	@echo "$(YELLOW)Warning: This will overwrite current data!$(NC)"
	@echo "Press Ctrl+C to cancel, or Enter to continue..."
	@read confirm
	docker run --rm -v hippo-data:/data -v $$(pwd):/backup alpine tar xzf /backup/$(BACKUP_FILE) -C /
	@echo "$(GREEN)Data restored from $(BACKUP_FILE)$(NC)"

volumes: ## List Docker volumes
	@echo "$(BLUE)Docker volumes:$(NC)"
	@docker volume ls | grep hippo

clean-volumes: ## Remove all volumes (WARNING: deletes all data!)
	@echo "$(RED)WARNING: This will delete all data!$(NC)"
	@echo "Press Ctrl+C to cancel, or type 'yes' to continue: "
	@read confirm && [ "$$confirm" = "yes" ]
	docker-compose down -v
	@echo "$(GREEN)Volumes removed$(NC)"

# Cleanup
clean: ## Clean up stopped containers and unused images
	@echo "$(BLUE)Cleaning up Docker resources...$(NC)"
	docker-compose down
	docker system prune -f
	@echo "$(GREEN)Cleanup complete$(NC)"

clean-all: ## Deep clean (removes all images, volumes, etc.)
	@echo "$(RED)WARNING: This will remove all Docker resources!$(NC)"
	@echo "Press Ctrl+C to cancel, or type 'yes' to continue: "
	@read confirm && [ "$$confirm" = "yes" ]
	docker-compose down -v
	docker system prune -af --volumes
	@echo "$(GREEN)Deep clean complete$(NC)"

# Build and deploy
rebuild: ## Rebuild images without cache and restart
	@echo "$(BLUE)Rebuilding images...$(NC)"
	docker-compose build --no-cache
	docker-compose up -d
	@echo "$(GREEN)Rebuild complete$(NC)"

update: ## Pull latest code and rebuild
	@echo "$(BLUE)Pulling latest changes...$(NC)"
	git pull origin main
	@echo "$(BLUE)Rebuilding images...$(NC)"
	docker-compose build
	docker-compose up -d
	@echo "$(GREEN)Update complete$(NC)"

# Setup
setup: ## Initial setup (copy .env and start services)
	@if [ ! -f .env ]; then \
		echo "$(BLUE)Creating .env from .env.example...$(NC)"; \
		cp .env.example .env; \
		echo "$(YELLOW)Please edit .env to customize your setup$(NC)"; \
		echo "$(YELLOW)Then run 'make up' to start services$(NC)"; \
	else \
		echo "$(GREEN).env already exists$(NC)"; \
		echo "$(BLUE)Starting services...$(NC)"; \
		docker-compose up -d; \
		echo "$(GREEN)Setup complete!$(NC)"; \
	fi

# Shell access
shell: ## Open shell in hippo-web container
	docker-compose exec hippo-web /bin/bash

shell-root: ## Open root shell in hippo-web container
	docker-compose exec -u root hippo-web /bin/bash

# Inspect
inspect-web: ## Inspect hippo-web container
	docker-compose exec hippo-web ls -la /data

inspect-db: ## Show database info
	docker-compose exec hippo-web sqlite3 /data/hippo.db ".tables"

# Port forwarding (for development)
tunnel: ## Create SSH tunnel for remote access (use PORT=3000 REMOTE_PORT=8080 REMOTE_HOST=example.com)
	@echo "$(BLUE)Creating SSH tunnel...$(NC)"
	@ssh -N -L $(PORT):localhost:$(REMOTE_PORT) $(REMOTE_HOST)
