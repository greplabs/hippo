# Hippo Docker Quick Start

Get Hippo running in Docker in under 5 minutes.

## Prerequisites

- Docker Engine 20.10+
- Docker Compose 2.0+
- 2GB free disk space
- Ports 3000, 6333, 6334 available

## Installation

### Option 1: Using Make (Recommended)

```bash
# 1. Clone repository
git clone https://github.com/greplabs/hippo.git
cd hippo

# 2. Initial setup (creates .env and starts services)
make setup

# 3. Edit .env to add your directories (optional)
nano .env

# 4. Restart if you edited .env
make restart
```

### Option 2: Manual Setup

```bash
# 1. Clone repository
git clone https://github.com/greplabs/hippo.git
cd hippo

# 2. Create environment file
cp .env.example .env

# 3. Edit configuration (optional)
nano .env

# 4. Start services
docker-compose up -d
```

## Quick Commands

```bash
# View status
make ps                    # or: docker-compose ps

# View logs
make logs                  # or: docker-compose logs -f

# Check health
make health               # or: curl http://localhost:3000/health

# Stop services
make down                 # or: docker-compose down

# Restart services
make restart              # or: docker-compose restart
```

## Accessing Services

After starting:

- **Hippo Web**: http://localhost:3000
- **Qdrant Dashboard**: http://localhost:6333/dashboard
- **Health Check**: http://localhost:3000/health

## Adding Your Files

Edit `.env` and add paths to index:

```bash
HIPPO_INDEX_PATH_1=/Users/yourname/Documents
HIPPO_INDEX_PATH_2=/Users/yourname/Photos
HIPPO_INDEX_PATH_3=/Users/yourname/Downloads
```

Then edit `docker-compose.yml` volumes section:

```yaml
volumes:
  - /Users/yourname/Documents:/mnt/index/documents:ro
  - /Users/yourname/Photos:/mnt/index/photos:ro
  - /Users/yourname/Downloads:/mnt/index/downloads:ro
```

Restart: `make restart`

## Testing

```bash
# Test API endpoints
make test-api

# Expected output:
# {
#   "status": "ok",
#   "version": "0.1.0"
# }
```

## Development Mode

For code development with hot reload:

```bash
make dev
```

This will:
- Mount source code as volumes
- Auto-rebuild on file changes
- Enable debug logging
- Start in foreground (Ctrl+C to stop)

## Troubleshooting

### Port already in use

```bash
# Change port in .env
echo "HIPPO_PORT=3001" >> .env
make restart
```

### Permission denied

```bash
# Fix permissions
sudo chown -R $USER:$USER .
```

### Out of memory

```bash
# Increase limits in .env
echo "HIPPO_MEM_LIMIT=4G" >> .env
make restart
```

### Container won't start

```bash
# Check logs
make logs-web

# Rebuild from scratch
make rebuild
```

## Data Management

### Backup

```bash
make backup
# Backups saved to ./backups/
```

### Restore

```bash
make restore BACKUP_FILE=backups/hippo-data-20250120-120000.tar.gz
```

### Reset

```bash
# WARNING: Deletes all data!
make clean-volumes
make up
```

## Production Deployment

### Recommended Settings

Edit `.env`:

```bash
# Logging
RUST_LOG=warn
RUST_BACKTRACE=0

# Resources
HIPPO_CPU_LIMIT=4.0
HIPPO_MEM_LIMIT=4G
QDRANT_CPU_LIMIT=2.0
QDRANT_MEM_LIMIT=2G

# Enable all features
QDRANT_ENABLED=true
```

### With Nginx Reverse Proxy

Create `/etc/nginx/sites-available/hippo`:

```nginx
server {
    listen 80;
    server_name hippo.yourdomain.com;

    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

Enable and restart Nginx:

```bash
sudo ln -s /etc/nginx/sites-available/hippo /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

### SSL with Let's Encrypt

```bash
sudo certbot --nginx -d hippo.yourdomain.com
```

## Resource Usage

Typical resource usage:

| Service | CPU | Memory | Disk |
|---------|-----|--------|------|
| hippo-web | ~5% | ~512MB | ~100MB |
| qdrant | ~2% | ~256MB | ~500MB |
| **Total** | **~7%** | **~768MB** | **~600MB** |

## Monitoring

```bash
# Real-time stats
make stats

# Container health
make health

# View specific logs
make logs-web
make logs-qdrant
```

## Updating

```bash
# Pull latest changes and rebuild
make update
```

## Uninstalling

```bash
# Stop services
make down

# Remove data (optional)
make clean-volumes

# Remove all Docker resources
make clean-all
```

## Getting Help

- **Documentation**: See [DOCKER.md](DOCKER.md) for detailed guide
- **Make commands**: Run `make help` for all available commands
- **Issues**: https://github.com/greplabs/hippo/issues

## Next Steps

1. Index your files: Add paths in `.env` and `docker-compose.yml`
2. Install Ollama for AI features: https://ollama.ai
3. Configure AI models in `.env`
4. Set up backups: `make backup` (schedule with cron)
5. Enable HTTPS for production

## Common Workflows

### Daily Development

```bash
make dev                  # Start dev environment
# Edit code...
# Watch automatic rebuild
Ctrl+C                    # Stop when done
```

### Production Updates

```bash
make backup              # Backup data first
make update              # Pull and rebuild
make health              # Verify health
```

### Debugging Issues

```bash
make logs-web            # Check web server logs
make shell               # Open shell in container
make inspect-db          # Check database
```

## Performance Tips

1. **Increase workers**: Set `TOKIO_WORKER_THREADS=8` in `.env`
2. **More memory**: Set `HIPPO_MEM_LIMIT=4G` for large collections
3. **SSD storage**: Store volumes on SSD for better performance
4. **Read-only mounts**: Keep indexed paths as `:ro` for safety

## Security Best Practices

1. Keep `.env` out of git (already in `.gitignore`)
2. Use read-only mounts for indexed paths (`:ro`)
3. Run behind reverse proxy with SSL in production
4. Regularly backup data
5. Monitor logs for suspicious activity
6. Keep Docker and images updated

## FAQ

**Q: Can I run this on a server without X11?**
A: Yes! The Docker deployment is headless and doesn't require a display server.

**Q: How do I index network drives?**
A: Mount the network drive on your host, then add it to volumes in `docker-compose.yml`.

**Q: Can I run multiple instances?**
A: Yes, but you'll need to change ports and volume names for each instance.

**Q: Does this work on Windows/Mac?**
A: Yes, Docker works on all platforms. Adjust paths in `.env` for your OS.

**Q: How do I connect to Ollama running on my host?**
A: Use `OLLAMA_HOST=http://host.docker.internal:11434` in `.env`.

**Q: Can I disable Qdrant?**
A: Yes, set `QDRANT_ENABLED=false` in `.env`. The app will work without semantic search.

---

**Ready to explore your digital memory?**

Start with: `make setup` and visit http://localhost:3000
