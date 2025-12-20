# Docker Deployment Files - Summary

This document lists all Docker-related files created for the Hippo web deployment.

## Created Files

### Core Docker Configuration

1. **docker-compose.yml** (2.8 KB)
   - Main Docker Compose configuration
   - Defines hippo-web and qdrant services
   - Volume mounts for data persistence
   - Network configuration
   - Health checks and resource limits

2. **Dockerfile.web** (6.7 KB)
   - Multi-stage build for hippo-web server
   - Build stage: Rust 1.85 with all dependencies
   - Runtime stage: Debian bookworm-slim
   - Creates hippo-web placeholder service (to be implemented)
   - Non-root user (uid 1000)
   - Exposes port 3000

3. **docker-compose.dev.yml** (1.9 KB)
   - Development overrides for docker-compose.yml
   - Source code volume mounts for hot reload
   - cargo-watch for automatic recompilation
   - Debug logging enabled
   - Removes resource limits
   - Local data directories

### Environment Configuration

4. **.env.example** (3.5 KB)
   - Template for environment variables
   - Server configuration (host, port)
   - Database paths
   - Qdrant settings
   - File indexing paths
   - AI/Ollama integration
   - Logging configuration
   - Resource limits

### Documentation

5. **DOCKER.md** (10 KB)
   - Comprehensive Docker deployment guide
   - Environment configuration details
   - Docker commands reference
   - Development setup instructions
   - Health checks and monitoring
   - Data persistence and backup
   - Troubleshooting guide
   - Production deployment recommendations
   - API endpoints documentation

6. **QUICKSTART-DOCKER.md** (6.5 KB)
   - Quick start guide (get running in 5 minutes)
   - Step-by-step installation
   - Common commands
   - Quick troubleshooting
   - FAQ section

### Build Automation

7. **Makefile** (6.4 KB)
   - Simplified Docker commands
   - `make setup` - Initial setup
   - `make up/down` - Start/stop services
   - `make logs` - View logs
   - `make health` - Check service health
   - `make backup/restore` - Data management
   - `make dev` - Development mode
   - `make clean` - Cleanup
   - Color-coded output

### CI/CD

8. **.github/workflows/docker.yml** (Updated)
   - Added build-docker-web job for Dockerfile.web
   - Builds both CLI and Web images
   - Multi-platform builds (linux/amd64, linux/arm64)
   - Trivy security scanning
   - Docker Compose integration tests
   - Pushes to GitHub Container Registry

### Updated Files

9. **.dockerignore** (Updated)
   - Added hippo-tauri exclusion (web build doesn't need it)
   - Added dev-data/ and dev-qdrant-data/
   - Added docker-compose.dev.yml

10. **.gitignore** (Updated)
    - Added dev-data/ and dev-qdrant-data/ directories

11. **README.md** (Updated)
    - Added Docker deployment section
    - Docker prerequisites
    - Quick start for web deployment
    - Configuration examples
    - Health check instructions
    - Links to Docker documentation

## File Structure

```
hippov20/
├── docker-compose.yml              # Main compose config
├── docker-compose.dev.yml          # Dev overrides
├── Dockerfile                      # CLI image (existing)
├── Dockerfile.web                  # Web server image (new)
├── .env.example                    # Environment template
├── Makefile                        # Build automation
├── DOCKER.md                       # Detailed guide
├── QUICKSTART-DOCKER.md           # Quick start
├── DOCKER-FILES-SUMMARY.md        # This file
├── .dockerignore                   # Build exclusions
├── .gitignore                      # Git exclusions
└── .github/workflows/docker.yml   # CI/CD pipeline

# Generated at runtime (not in git)
├── .env                            # User config (from .env.example)
├── dev-data/                       # Dev database
└── dev-qdrant-data/               # Dev vector DB
```

## Usage Quick Reference

### First Time Setup

```bash
make setup                          # Copy .env and start services
# or
cp .env.example .env
docker-compose up -d
```

### Daily Operations

```bash
make up                             # Start services
make down                           # Stop services
make logs                           # View logs
make restart                        # Restart services
make health                         # Check health
```

### Development

```bash
make dev                            # Start dev mode with hot reload
make dev-build                      # Build and start dev
```

### Data Management

```bash
make backup                         # Backup database
make restore BACKUP_FILE=...        # Restore from backup
make volumes                        # List volumes
```

### Maintenance

```bash
make rebuild                        # Rebuild without cache
make update                         # Pull code and rebuild
make clean                          # Clean up resources
```

## Docker Images

### hippo-web (Dockerfile.web)
- Base: rust:1.85-slim-bookworm (build), debian:bookworm-slim (runtime)
- Size: ~300MB (estimated)
- Port: 3000
- User: hippo (uid 1000)
- Health check: GET /health

### qdrant (Official)
- Image: qdrant/qdrant:v1.12.2
- Size: ~200MB
- Ports: 6333 (HTTP), 6334 (gRPC)
- Persistent volume: qdrant-data

## Network Architecture

```
┌─────────────────────────────────────────┐
│  Docker Host                             │
│                                          │
│  ┌────────────────────────────────────┐ │
│  │  hippo-network (bridge)            │ │
│  │                                    │ │
│  │  ┌──────────────┐  ┌───────────┐  │ │
│  │  │  hippo-web   │  │  qdrant   │  │ │
│  │  │  :3000       │──│  :6334    │  │ │
│  │  └──────┬───────┘  └─────┬─────┘  │ │
│  └─────────┼────────────────┼────────┘ │
│            │                │          │
│  ┌─────────┼────────────────┼────────┐ │
│  │         │                │        │ │
│  │  ┌──────▼─────┐   ┌──────▼─────┐ │ │
│  │  │ hippo-data │   │qdrant-data │ │ │
│  │  └────────────┘   └────────────┘ │ │
│  └─────────────────────────────────┘ │
└───────────┬──────────────────────────┘
            │
      ┌─────▼─────┐
      │  Port 3000 │ (exposed to host)
      │  Port 6333 │ (exposed to host)
      └───────────┘
```

## Environment Variables Reference

### Required
- `HIPPO_HOST` - Server bind address (default: 0.0.0.0)
- `HIPPO_PORT` - Server port (default: 3000)
- `HIPPO_DB_PATH` - Database path (default: /data/hippo.db)

### Optional
- `QDRANT_URL` - Qdrant connection URL
- `QDRANT_ENABLED` - Enable Qdrant (default: true)
- `OLLAMA_HOST` - Ollama API URL
- `RUST_LOG` - Log level (default: info)
- `TOKIO_WORKER_THREADS` - Worker threads (default: 4)

### Resource Limits
- `HIPPO_CPU_LIMIT` - Max CPU cores (default: 2.0)
- `HIPPO_MEM_LIMIT` - Max memory (default: 2G)
- `QDRANT_CPU_LIMIT` - Qdrant CPU (default: 1.0)
- `QDRANT_MEM_LIMIT` - Qdrant memory (default: 1G)

## Security Features

1. **Non-root user**: Containers run as user `hippo` (uid 1000)
2. **Read-only mounts**: Indexed files mounted as `:ro`
3. **Network isolation**: Services on internal bridge network
4. **Health checks**: Automatic health monitoring
5. **Security scanning**: Trivy scans in CI/CD
6. **Resource limits**: CPU and memory constraints

## Next Steps

The hippo-web service is currently a placeholder that needs implementation:

1. **Create hippo-web crate**: Implement web server with Axum
2. **Add API endpoints**: Search, sources, tags, etc.
3. **Web UI**: HTML/JS frontend or serve Tauri UI
4. **Authentication**: Add JWT or session-based auth
5. **WebSockets**: Real-time updates for indexing progress
6. **Metrics**: Prometheus metrics endpoint

## Contributing

When adding Docker features:
1. Update `.env.example` with new variables
2. Document changes in DOCKER.md
3. Update Makefile if adding new commands
4. Test with `docker-compose up`
5. Test dev mode with `make dev`
6. Update CI/CD if needed

## Support

- Full guide: [DOCKER.md](DOCKER.md)
- Quick start: [QUICKSTART-DOCKER.md](QUICKSTART-DOCKER.md)
- Issues: https://github.com/greplabs/hippo/issues
