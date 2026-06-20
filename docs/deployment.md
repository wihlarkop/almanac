# Deployment Guide

Almanac ships as a single static binary and a pre-built Docker image. The catalog (models, providers, aliases) is embedded in the image — no database required.

- **Docker image:** `ghcr.io/wihlarkop/almanac`
- **Health check:** `GET /api/v1/health`
- **Default port:** `8080`

---

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `PORT` | `8080` | TCP port the server listens on |
| `DATA_DIR` | `.` | Directory containing `models/`, `providers/`, `aliases.yaml` |
| `RUST_LOG` | `almanac_server=info,tower_http=info` | Log level filter (uses `tracing` syntax) |
| `LOG_FORMAT` | _(unset)_ | Set to `json` for structured JSON logs (recommended in production) |
| `RATE_LIMIT_RPS` | _(unset)_ | Max requests per second per IP. Unset = rate limiting disabled |
| `RATE_LIMIT_BACKEND` | `memory` | `memory` or `redis`. Falls back to memory if Redis is not configured |
| `REDIS_URL` | _(unset)_ | Redis connection string for redis-backed rate limiting and caching |
| `TRUST_PROXY_HEADERS` | _(unset)_ | Set to `true` only behind a trusted proxy/load balancer |
| `CACHE_BACKEND` | `none` | `none`, `memory`, or `redis` |
| `CACHE_TTL_SECS` | `300` | Cache time-to-live in seconds |
| `CATALOG_INCLUDE_PROVIDERS` | _(unset)_ | Comma-separated provider IDs to include (e.g. `openai,anthropic`) |
| `CATALOG_EXCLUDE_PROVIDERS` | _(unset)_ | Comma-separated provider IDs to exclude |
| `CATALOG_INCLUDE_MODELS` | _(unset)_ | Comma-separated model IDs to include |
| `CATALOG_EXCLUDE_MODELS` | _(unset)_ | Comma-separated model IDs to exclude |
| `CATALOG_SCOPE_FILE` | _(unset)_ | Path to a YAML scope file for fine-grained catalog filtering |

---

## Render

Render is the reference deployment platform. The live instance runs here.

### New deployment

1. Go to [dashboard.render.com](https://dashboard.render.com) → **New** → **Web Service**
2. Connect your GitHub repository
3. Set the following:

| Setting | Value |
|---|---|
| **Runtime** | Docker |
| **Branch** | `main` |
| **Health Check Path** | `/api/v1/health` |
| **Port** | `8080` |

4. Add environment variables:

```
LOG_FORMAT=json
RUST_LOG=almanac_server=info,tower_http=info,server=info
RATE_LIMIT_RPS=10
TRUST_PROXY_HEADERS=true
```

5. Click **Deploy**

### Auto-deploy on release

To deploy only on tagged releases (not every push to main):

1. Go to **Settings** → **Build & Deploy**
2. Set **Auto-Deploy** to **No**
3. Use the `RENDER_DEPLOY_HOOK_URL` secret in `.github/workflows/release.yml` — it triggers a deploy automatically when a `v*` tag is pushed

---

## Fly.io

```bash
# Install flyctl
curl -L https://fly.io/install.sh | sh

# Create app (first time only)
fly launch --image ghcr.io/wihlarkop/almanac:latest --name almanac --port 8080

# Set env vars
fly secrets set LOG_FORMAT=json
fly secrets set RATE_LIMIT_RPS=10 TRUST_PROXY_HEADERS=true

# Deploy
fly deploy --image ghcr.io/wihlarkop/almanac:latest
```

Update to a new release:

```bash
fly deploy --image ghcr.io/wihlarkop/almanac:v0.27.0
```

---

## Railway

1. Go to [railway.app](https://railway.app) → **New Project** → **Deploy from image**
2. Enter image: `ghcr.io/wihlarkop/almanac:latest`
3. Set environment variables under **Variables**:

```
PORT=8080
LOG_FORMAT=json
RATE_LIMIT_RPS=10
TRUST_PROXY_HEADERS=true
```

4. Railway auto-detects port `8080` from the `EXPOSE` directive

---

## Google Cloud Run

```bash
gcloud run deploy almanac \
  --image ghcr.io/wihlarkop/almanac:latest \
  --platform managed \
  --region us-central1 \
  --port 8080 \
  --allow-unauthenticated \
  --set-env-vars LOG_FORMAT=json,RATE_LIMIT_RPS=10,TRUST_PROXY_HEADERS=true
```

Cloud Run scales to zero by default. To keep one instance warm:

```bash
gcloud run services update almanac --min-instances 1
```

---

## Docker (self-hosted / VPS)

### Run directly

```bash
docker run -d \
  --name almanac \
  -p 8080:8080 \
  -e LOG_FORMAT=json \
  -e RUST_LOG=almanac_server=info,tower_http=info,server=info \
  -e RATE_LIMIT_RPS=10 \
  --restart unless-stopped \
  ghcr.io/wihlarkop/almanac:latest
```

### Docker Compose

```yaml
services:
  almanac:
    image: ghcr.io/wihlarkop/almanac:latest
    ports:
      - "8080:8080"
    environment:
      LOG_FORMAT: json
      RUST_LOG: almanac_server=info,tower_http=info,server=info
      RATE_LIMIT_RPS: "10"
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "wget", "-qO-", "http://localhost:8080/api/v1/health"]
      interval: 30s
      timeout: 5s
      retries: 3
```

```bash
docker compose up -d
```

---

## Build from Source

Requires Rust 1.96+.

```bash
git clone https://github.com/wihlarkop/almanac.git
cd almanac
cargo build --release -p almanac-server
./target/release/server
```

The binary expects `models/`, `providers/`, `aliases.yaml`, and `schema/` in the working directory (or set `DATA_DIR` to point elsewhere).

---

## Verifying a Deployment

```bash
curl https://your-instance/api/v1/health
```

Expected response:

```json
{
  "success": true,
  "message": "OK",
  "data": {
    "status": "ok",
    "version": "0.27.0",
    "total_models": 646,
    "total_providers": 64,
    "total_aliases": 273
  }
}
```

---

## Production Checklist

- [ ] `LOG_FORMAT=json` set (structured logs for log aggregators)
- [ ] Health check path configured (`/api/v1/health`)
- [ ] `RATE_LIMIT_RPS` set if the instance is public-facing
- [ ] `TRUST_PROXY_HEADERS=true` set only when the service is behind a trusted proxy/load balancer
- [ ] Deploy triggered by `v*` tags only, not every push to main
