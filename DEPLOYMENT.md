# Hippo Deployment Guide

This guide covers deploying Hippo in various configurations: desktop app, web server, and static demo.

## Table of Contents

- [Desktop App Deployment](#desktop-app-deployment)
- [Web Server Deployment](#web-server-deployment)
- [Static Demo Deployment](#static-demo-deployment)
- [Docker Deployment](#docker-deployment)

---

## Desktop App Deployment

### Building Desktop App with Tauri

The desktop app is the primary way to use Hippo with full file system access.

#### Prerequisites

- Rust 1.70+
- Node.js 16+ (optional, UI is pre-built)
- Platform-specific dependencies:
  - **macOS**: Xcode Command Line Tools
  - **Windows**: MSVC Build Tools
  - **Linux**: WebKit2GTK, libappindicator

#### Build

```bash
cd hippo-tauri
cargo build --release
```

#### Run

```bash
cd hippo-tauri
cargo run
```

#### Create Installer

```bash
cd hippo-tauri
cargo tauri build
```

This creates platform-specific installers in `hippo-tauri/target/release/bundle/`.

### Distribution

- **macOS**: `.dmg` and `.app` in `bundle/dmg/`
- **Windows**: `.msi` installer in `bundle/msi/`
- **Linux**: `.deb`, `.rpm`, `.AppImage` in respective `bundle/` subdirectories

---

## Web Server Deployment

Deploy Hippo as a web service with the REST API backend.

### Option 1: Docker (Recommended)

See [Docker Deployment](#docker-deployment) below.

### Option 2: Standalone Binary

#### Build

```bash
cd hippo-web
cargo build --release
```

#### Run

```bash
cd hippo-web

# Set environment variables
export HIPPO_HOST=0.0.0.0
export HIPPO_PORT=3000
export HIPPO_DATA_DIR=/path/to/data

# Start server
./target/release/hippo-web
```

#### Systemd Service (Linux)

Create `/etc/systemd/system/hippo-web.service`:

```ini
[Unit]
Description=Hippo Web API Server
After=network.target

[Service]
Type=simple
User=hippo
Group=hippo
WorkingDirectory=/opt/hippo
Environment="HIPPO_HOST=0.0.0.0"
Environment="HIPPO_PORT=3000"
Environment="HIPPO_DATA_DIR=/var/lib/hippo"
ExecStart=/opt/hippo/hippo-web
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable hippo-web
sudo systemctl start hippo-web
sudo systemctl status hippo-web
```

#### Nginx Reverse Proxy

```nginx
server {
    listen 80;
    server_name hippo.example.com;

    # Redirect HTTP to HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name hippo.example.com;

    ssl_certificate /etc/ssl/certs/hippo.example.com.crt;
    ssl_certificate_key /etc/ssl/private/hippo.example.com.key;

    # Security headers
    add_header X-Frame-Options "DENY" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_cache_bypass $http_upgrade;
    }
}
```

---

## Static Demo Deployment

Deploy a static demo version with sample data for showcasing Hippo without a backend.

### Vercel Deployment (Recommended)

#### Quick Deploy

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https://github.com/greplabs/hippo)

#### Manual Deployment

```bash
# Install Vercel CLI
npm install -g vercel

# From project root
vercel

# Production deployment
vercel --prod
```

The `vercel.json` configuration is already set up to serve the `demo/` directory.

#### Configuration

The demo uses static JSON files in `demo/api/` for mock data. Customize by editing:

- `demo/api/stats.json` - Index statistics
- `demo/api/sources.json` - Sample sources
- `demo/api/tags.json` - Available tags
- `demo/api/search.json` - Search results

#### Environment Variables

No environment variables needed for static demo. To connect to a live backend, set:

```env
HIPPO_API_URL=https://your-hippo-api.com
```

### Netlify Deployment

#### Create `netlify.toml`

```toml
[build]
  publish = "demo"
  command = "echo 'Static site - no build needed'"

[[redirects]]
  from = "/api/*"
  to = "/api/:splat"
  status = 200

[[headers]]
  for = "/*"
  [headers.values]
    X-Frame-Options = "DENY"
    X-Content-Type-Options = "nosniff"
    X-XSS-Protection = "1; mode=block"

[[headers]]
  for = "/api/*"
  [headers.values]
    Content-Type = "application/json"
    Access-Control-Allow-Origin = "*"
```

#### Deploy

```bash
# Install Netlify CLI
npm install -g netlify-cli

# Deploy
netlify deploy --dir=demo --prod
```

Or use the Netlify web interface:
1. Connect your GitHub repository
2. Set publish directory to `demo`
3. Leave build command empty
4. Deploy

### GitHub Pages

```bash
# Enable GitHub Pages in repo settings
# Set source to: Deploy from a branch
# Branch: main
# Folder: /demo

# Or use gh-pages branch
git subtree push --prefix demo origin gh-pages
```

### Cloudflare Pages

1. Connect GitHub repository
2. Set build output directory to `demo`
3. Leave build command empty
4. Deploy

### Self-Hosted Static

Use any web server:

```bash
# Python
cd demo
python3 -m http.server 8080

# Node.js
npx http-server demo -p 8080

# PHP
cd demo
php -S localhost:8080

# Nginx
server {
    listen 80;
    server_name demo.hippo.local;
    root /path/to/hippo/demo;
    index index.html;

    location / {
        try_files $uri $uri/ /index.html;
    }

    location /api {
        try_files $uri $uri.json =404;
    }
}
```

---

## Docker Deployment

### Quick Start

```bash
# Clone repository
git clone https://github.com/greplabs/hippo.git
cd hippo

# Configure environment
cp .env.example .env
# Edit .env to set your preferences

# Start services
docker-compose up -d

# View logs
docker-compose logs -f hippo-web

# Stop services
docker-compose down
```

### Services

The Docker Compose stack includes:

- **hippo-web**: Web API server (port 3000)
- **qdrant**: Vector database for semantic search (ports 6333, 6334)

### Configuration

Edit `.env` file:

```bash
# Server Configuration
HIPPO_HOST=0.0.0.0
HIPPO_PORT=3000
HIPPO_DATA_DIR=/data

# Resource Limits
HIPPO_CPU_LIMIT=2.0
HIPPO_MEM_LIMIT=2G

# Indexing Paths (add as many as needed)
HIPPO_INDEX_PATH_1=/mnt/documents
HIPPO_INDEX_PATH_2=/mnt/photos
HIPPO_INDEX_PATH_3=/mnt/videos
```

### Volume Mounts

Update `docker-compose.yml` to mount your directories:

```yaml
services:
  hippo-web:
    volumes:
      - hippo-data:/data
      - /path/to/your/documents:/mnt/documents:ro
      - /path/to/your/photos:/mnt/photos:ro
      - /path/to/your/videos:/mnt/videos:ro
```

### Health Checks

```bash
# Check Hippo Web
curl http://localhost:3000/api/health

# Check Qdrant
curl http://localhost:6333/
```

### Production Deployment

1. **Use specific versions** instead of `latest`:
   ```yaml
   image: ghcr.io/greplabs/hippo-web:v0.1.0
   ```

2. **Enable TLS** with Let's Encrypt:
   ```bash
   # Add Caddy or Traefik as reverse proxy
   ```

3. **Set up backups**:
   ```bash
   # Backup volumes
   docker run --rm -v hippo-data:/data -v $(pwd):/backup \
     alpine tar czf /backup/hippo-backup-$(date +%Y%m%d).tar.gz /data
   ```

4. **Monitor resources**:
   ```bash
   docker stats
   ```

### Docker Build from Source

```bash
# Build web server
docker build -f Dockerfile.web -t hippo-web:local .

# Build full stack
docker-compose build

# Run custom build
docker-compose up -d
```

---

## Performance Tuning

### Database Optimization

```bash
# Vacuum SQLite database
sqlite3 ~/.local/share/Hippo/hippo.db "VACUUM;"

# Analyze for better query planning
sqlite3 ~/.local/share/Hippo/hippo.db "ANALYZE;"
```

### Indexing Performance

```bash
# Adjust batch size in .env
HIPPO_INDEX_BATCH_SIZE=100

# Adjust worker threads
HIPPO_INDEX_WORKERS=4
```

### Caching

```bash
# Enable response caching (nginx)
proxy_cache_path /var/cache/nginx levels=1:2 keys_zone=hippo_cache:10m max_size=1g;
proxy_cache hippo_cache;
proxy_cache_valid 200 1h;
```

---

## Monitoring & Logging

### Logs

```bash
# Docker logs
docker-compose logs -f hippo-web

# Systemd logs
journalctl -u hippo-web -f

# Application logs
tail -f /var/log/hippo/hippo-web.log
```

### Metrics

```bash
# Prometheus endpoint (future)
curl http://localhost:3000/metrics

# Stats endpoint
curl http://localhost:3000/api/stats
```

---

## Security Best Practices

1. **Use HTTPS** in production
2. **Enable authentication** (not yet implemented - use reverse proxy auth)
3. **Restrict CORS** origins in production
4. **Use firewall** rules to limit access
5. **Regular backups** of database and indices
6. **Keep dependencies updated**
7. **Use non-root user** for running services
8. **Limit file system access** to necessary directories only

---

## Troubleshooting

### Desktop App Won't Start

```bash
# Check Rust version
rustc --version

# Rebuild from scratch
cd hippo-tauri
cargo clean
cargo build --release
```

### Web Server Connection Refused

```bash
# Check if service is running
systemctl status hippo-web

# Check port binding
netstat -tlnp | grep 3000

# Check firewall
sudo ufw status
```

### Docker Container Issues

```bash
# Check container status
docker ps -a

# View container logs
docker logs hippo-web

# Restart container
docker-compose restart hippo-web

# Rebuild container
docker-compose up -d --build
```

### Database Locked

```bash
# Stop all services
docker-compose down

# Remove lock file
rm ~/.local/share/Hippo/hippo.db-shm
rm ~/.local/share/Hippo/hippo.db-wal

# Restart
docker-compose up -d
```

---

## Scaling

### Horizontal Scaling

Hippo Web can be scaled horizontally with load balancing:

```nginx
upstream hippo_backend {
    least_conn;
    server hippo1:3000;
    server hippo2:3000;
    server hippo3:3000;
}

server {
    location / {
        proxy_pass http://hippo_backend;
    }
}
```

### Vertical Scaling

Increase resources in Docker:

```yaml
deploy:
  resources:
    limits:
      cpus: '4.0'
      memory: 8G
```

---

## Support

- **Documentation**: See [README.md](README.md) and [CONTRIBUTING.md](CONTRIBUTING.md)
- **Issues**: https://github.com/greplabs/hippo/issues
- **Discussions**: https://github.com/greplabs/hippo/discussions

---

## License

MIT - see [LICENSE](LICENSE) file
