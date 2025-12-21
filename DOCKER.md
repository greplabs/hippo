# Hippo Docker Deployment Guide

This guide covers deploying Hippo as a web service using Docker and Docker Compose.

## Overview

The Docker setup includes:

- **hippo-web**: Web server with REST API (built from `Dockerfile.web`)
- **qdrant**: Vector database for semantic search (optional but recommended)
- Persistent volumes for data and configuration
- Health checks and resource limits
- Development configuration with hot reload

## Quick Start

```bash
# 1. Copy environment template
cp .env.example .env

# 2. Edit .env to customize your setup
nano .env

# 3. Start services
docker-compose up -d

# 4. View logs
docker-compose logs -f hippo-web

# 5. Access the web interface
open http://localhost:3000
```

## Environment Configuration

The `.env` file contains all configurable options:

### Server Settings

```bash
HIPPO_HOST=0.0.0.0        # Server bind address
HIPPO_PORT=3000           # Server port
```

### Database

```bash
HIPPO_DB_PATH=/data/hippo.db  # SQLite database path (inside container)
```

### Qdrant Vector Database

```bash
QDRANT_URL=http://qdrant:6334  # Qdrant connection URL
QDRANT_ENABLED=true            # Enable/disable Qdrant features
QDRANT_PORT=6333               # HTTP API port (exposed to host)
QDRANT_GRPC_PORT=6334          # gRPC API port (exposed to host)
```

### File Indexing Paths

Add paths to index by setting environment variables and mounting volumes:

```bash
# .env
HIPPO_INDEX_PATH_1=/Users/you/Documents
HIPPO_INDEX_PATH_2=/Users/you/Photos
HIPPO_INDEX_PATH_3=/Users/you/Downloads
```

Then update `docker-compose.yml`:

```yaml
volumes:
  - /Users/you/Documents:/mnt/index/documents:ro
  - /Users/you/Photos:/mnt/index/photos:ro
  - /Users/you/Downloads:/mnt/index/downloads:ro
```

### AI Features (Ollama)

If you have Ollama running on your host machine:

```bash
OLLAMA_HOST=http://host.docker.internal:11434
OLLAMA_CHAT_MODEL=qwen2:0.5b
OLLAMA_EMBED_MODEL=nomic-embed-text
```

### Logging

```bash
RUST_LOG=info              # Log level (error, warn, info, debug, trace)
RUST_BACKTRACE=0           # Enable backtraces (0=off, 1=on, full=full)
```

### Resource Limits

```bash
# Hippo Web Service
HIPPO_CPU_LIMIT=2.0        # Max CPU cores
HIPPO_MEM_LIMIT=2G         # Max memory
HIPPO_CPU_RESERVE=0.5      # Reserved CPU
HIPPO_MEM_RESERVE=512M     # Reserved memory

# Qdrant Service
QDRANT_CPU_LIMIT=1.0
QDRANT_MEM_LIMIT=1G
QDRANT_CPU_RESERVE=0.25
QDRANT_MEM_RESERVE=256M
```

## Docker Commands

### Starting Services

```bash
# Start in foreground
docker-compose up

# Start in background (daemon mode)
docker-compose up -d

# Start specific service
docker-compose up hippo-web
```

### Viewing Logs

```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f hippo-web

# Last 100 lines
docker-compose logs --tail=100 hippo-web
```

### Stopping Services

```bash
# Stop all services
docker-compose down

# Stop but keep volumes
docker-compose stop

# Stop and remove volumes (data will be lost!)
docker-compose down -v
```

### Rebuilding

```bash
# Rebuild images
docker-compose build

# Rebuild without cache
docker-compose build --no-cache

# Rebuild and restart
docker-compose up -d --build
```

### Scaling

```bash
# Run multiple hippo-web instances (requires load balancer)
docker-compose up -d --scale hippo-web=3
```

## Development Setup

For development with hot reload:

```bash
# Start with development overrides
docker-compose -f docker-compose.yml -f docker-compose.dev.yml up

# This configuration:
# - Mounts source code as volumes
# - Uses cargo-watch for automatic recompilation
# - Enables debug logging
# - Removes resource limits
# - Exposes additional debug ports
```

### Development Features

- **Hot reload**: Code changes trigger automatic rebuild
- **Debug logging**: Detailed logs for troubleshooting
- **Source code mounting**: Edit code on host, runs in container
- **Cargo caching**: Faster rebuilds with cached dependencies

## Health Checks

All services include health checks:

```bash
# Check Hippo web server
curl http://localhost:3000/health

# Expected response:
# {"status":"ok","version":"0.1.0"}

# Check Qdrant
curl http://localhost:6333/

# Expected response:
# {"title":"qdrant - vector search engine","version":"1.12.2"}
```

### Docker Health Status

```bash
# View health status
docker-compose ps

# Output shows health status in STATE column:
# NAME              STATE
# hippo-web         Up (healthy)
# hippo-qdrant      Up (healthy)
```

## Data Persistence

Data is stored in Docker volumes:

```bash
# List volumes
docker volume ls | grep hippo

# Output:
# hippo-data         # SQLite database and app data
# qdrant-data        # Qdrant vector database
```

### Backup Data

```bash
# Backup hippo database
docker run --rm \
  -v hippo-data:/data \
  -v $(pwd):/backup \
  alpine tar czf /backup/hippo-backup.tar.gz /data

# Restore hippo database
docker run --rm \
  -v hippo-data:/data \
  -v $(pwd):/backup \
  alpine tar xzf /backup/hippo-backup.tar.gz -C /
```

### Backup Qdrant

```bash
# Backup Qdrant data
docker run --rm \
  -v qdrant-data:/qdrant \
  -v $(pwd):/backup \
  alpine tar czf /backup/qdrant-backup.tar.gz /qdrant

# Restore Qdrant data
docker run --rm \
  -v qdrant-data:/qdrant \
  -v $(pwd):/backup \
  alpine tar xzf /backup/qdrant-backup.tar.gz -C /
```

## Accessing Services

### Hippo Web Interface

- **URL**: http://localhost:3000
- **API Docs**: http://localhost:3000/api (when implemented)
- **Health Check**: http://localhost:3000/health

### Qdrant Dashboard

- **Web UI**: http://localhost:6333/dashboard
- **API**: http://localhost:6333
- **gRPC**: localhost:6334

### Ollama (on host)

- **URL**: http://localhost:11434
- **From container**: http://host.docker.internal:11434

## Troubleshooting

### Container won't start

```bash
# Check logs
docker-compose logs hippo-web

# Check container status
docker-compose ps

# Restart specific service
docker-compose restart hippo-web
```

### Database locked errors

```bash
# Stop all services
docker-compose down

# Remove database volume
docker volume rm hippo-data

# Restart
docker-compose up -d
```

### Out of disk space

```bash
# Clean up unused Docker resources
docker system prune -a

# Remove unused volumes
docker volume prune
```

### Permission errors

```bash
# Fix ownership (data is owned by uid 1000)
sudo chown -R 1000:1000 ./dev-data
```

### Qdrant connection errors

If Qdrant features are failing:

```bash
# Disable Qdrant temporarily
echo "QDRANT_ENABLED=false" >> .env
docker-compose restart hippo-web

# Or check Qdrant logs
docker-compose logs qdrant
```

### Port already in use

```bash
# Change port in .env
echo "HIPPO_PORT=3001" >> .env

# Restart
docker-compose up -d
```

## Performance Tuning

### Adjust Worker Threads

```bash
# Increase for multi-core systems
TOKIO_WORKER_THREADS=8
```

### Increase Memory Limits

```bash
# For large file collections
HIPPO_MEM_LIMIT=4G
QDRANT_MEM_LIMIT=2G
```

### Optimize Indexing

```bash
# Limit concurrent indexing
# (Set in application code, not environment)
```

## Production Deployment

### Recommended Configuration

```bash
# Use production log level
RUST_LOG=warn

# Disable debug features
RUST_BACKTRACE=0

# Set resource limits
HIPPO_CPU_LIMIT=4.0
HIPPO_MEM_LIMIT=4G
QDRANT_CPU_LIMIT=2.0
QDRANT_MEM_LIMIT=2G

# Enable Qdrant for semantic search
QDRANT_ENABLED=true
```

### Using with Reverse Proxy (Nginx)

```nginx
server {
    listen 80;
    server_name hippo.example.com;

    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### SSL/TLS with Traefik

```yaml
labels:
  - "traefik.enable=true"
  - "traefik.http.routers.hippo.rule=Host(`hippo.example.com`)"
  - "traefik.http.routers.hippo.entrypoints=websecure"
  - "traefik.http.routers.hippo.tls.certresolver=letsencrypt"
```

## Monitoring

### Container Stats

```bash
# Real-time resource usage
docker stats hippo-web hippo-qdrant

# Output:
# CONTAINER     CPU %    MEM USAGE / LIMIT    MEM %    NET I/O
# hippo-web     5.2%     512MB / 2GB          25.6%    1.2MB / 3.4MB
# hippo-qdrant  2.1%     256MB / 1GB          25.6%    0.5MB / 1.2MB
```

### Export Metrics

Add Prometheus metrics endpoint (to be implemented):

```bash
# Access metrics
curl http://localhost:9090/metrics
```

## Security Considerations

1. **Read-only mounts**: Index paths are mounted read-only (`:ro`)
2. **Non-root user**: Container runs as user `hippo` (uid 1000)
3. **Network isolation**: Services communicate via internal network
4. **Environment files**: Keep `.env` out of version control
5. **Resource limits**: Prevent resource exhaustion attacks

## Updating

```bash
# Pull latest changes
git pull origin main

# Rebuild images
docker-compose build --no-cache

# Restart with new images
docker-compose up -d

# Clean up old images
docker image prune -a
```

## Uninstalling

```bash
# Stop services
docker-compose down

# Remove volumes (WARNING: deletes all data!)
docker volume rm hippo-data qdrant-data

# Remove images
docker rmi hippo-web qdrant/qdrant

# Clean up everything
docker system prune -a --volumes
```

## API Endpoints

The hippo-web service exposes the following endpoints:

### Health & Status

- `GET /health` - Health check
- `GET /api/stats` - Index statistics

### Search

- `POST /api/search` - Search memories
  ```json
  {
    "text": "vacation photos",
    "limit": 100,
    "offset": 0
  }
  ```

### Sources

- `GET /api/sources` - List indexed sources
- `POST /api/sources` - Add new source (to be implemented)
- `DELETE /api/sources/:id` - Remove source (to be implemented)

### Tags

- `GET /api/tags` - List all tags (to be implemented)
- `POST /api/memories/:id/tags` - Add tag (to be implemented)

## Next Steps

- Implement remaining API endpoints
- Add authentication/authorization
- Create web UI for hippo-web
- Add Prometheus metrics
- Support horizontal scaling
- Implement API documentation (Swagger/OpenAPI)

## Support

For issues and questions:

- GitHub Issues: https://github.com/greplabs/hippo/issues
- Documentation: https://github.com/greplabs/hippo/docs
