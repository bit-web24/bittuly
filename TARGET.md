# Bittuly — Production Distributed URL Shortener
## Development & Deployment Plan

> **Deployment Target: Kubernetes (K8s)**
> Local → `kind` cluster → Managed K8s (DigitalOcean DOKS)
>
> **Why Kubernetes?**
> - Industry standard for production microservices at every major company
> - Manifests are 100% portable: DOKS today, EKS/GKE tomorrow, zero rewrite
> - Forces you to solve real distributed problems: health probes, rolling updates,
>   HPA scaling, PDB availability guarantees, secrets management
> - The single most valuable infrastructure skill in backend engineering today
>
> **Why DigitalOcean DOKS?**
> - Managed control plane (free), you only pay for worker nodes
> - ~$72/month for a 3-node cluster (2 vCPU, 4 GB each) — cheapest managed K8s
> - Simple kubectl integration, no AWS/GCP account complexity needed to start

---

## 1. Goal

Build and deploy a **production-grade, horizontally scalable URL shortener** using a
microservices architecture. The system must handle:

- **10,000 redirects / second** at p99 < 15 ms (cache hit) / < 60 ms (cache miss)
- **1,000 shortening requests / minute** across the cluster
- **99.9 % availability** (≤ 8.7 hours downtime / year)
- **Zero-downtime deploys** via Kubernetes rolling updates
- **Automatic recovery** from single-node or single-pod failures

---

## 2. Architecture Overview

```
Internet (HTTPS only)
         │
         ▼
┌────────────────────────────────────────────────────────────────┐
│                  NGINX Ingress Controller                      │
│  • TLS termination (cert-manager + Let's Encrypt auto-renew)  │
│  • Rate limiting (per-IP, per-route annotations)              │
│  • JWT validation (auth_request to auth-service)              │
│  • Routes  /api/auth/**  →  auth-service                      │
│            /api/**       →  url-service                       │
│            /**           →  frontend (Nginx static)           │
└──────────────────────┬─────────────────────┬──────────────────┘
           (K8s ClusterIP)       (K8s ClusterIP)
                       │                     │
                       ▼                     ▼
            ┌──────────────────┐   ┌──────────────────────┐
            │   auth-service   │   │     url-service       │
            │   Rust + Axum    │   │     Rust + Axum       │
            │   min 2 pods     │   │     min 3 pods        │
            │   HPA → max 5    │   │     HPA → max 10      │
            └────────┬─────────┘   └──────────┬────────────┘
                     │                         │
                     ▼                         ▼
            ┌──────────────┐         ┌──────────────────┐
            │  PgBouncer   │         │    PgBouncer      │
            │  (pooler)    │         │    (pooler)       │
            └──────┬───────┘         └────────┬──────────┘
                   │                          │
                   ▼                          ▼
            ┌──────────────┐         ┌──────────────────┐
            │  pg-auth     │         │    pg-urls        │
            │  Postgres 17 │         │    Postgres 17    │
            │  users, otps │         │    urls           │
            └──────────────┘         └──────────────────┘

    ┌────────────────────────┐     ┌──────────────────────────┐
    │  RabbitMQ (3-node)     │     │  Redis Sentinel           │
    │  user.events exchange  │     │  1 master + 2 replicas   │
    │  url.events exchange   │     │  cache + rate-limit store │
    └────────────────────────┘     └──────────────────────────┘

    ┌────────────┐    ┌──────────┐    ┌────────┐    ┌────────┐
    │ Prometheus │    │ Grafana  │    │  Loki  │    │ Jaeger │
    │ (metrics)  │    │(dashbrd) │    │ (logs) │    │(traces)│
    └────────────┘    └──────────┘    └────────┘    └────────┘
```

---

## 3. Services

### 3.1  auth-service  (port 3001, internal only)

| Property | Value |
|---|---|
| Database | `pg-auth` — tables: `users`, `otps` |
| Replicas | min 2, max 5 (HPA on CPU > 70%) |
| CPU | 100m request / 500m limit |
| Memory | 64 Mi request / 256 Mi limit |

**Owns:**
- `POST /api/auth/signup`
- `POST /api/auth/login`
- `POST /api/auth/verify-otp`
- `DELETE /api/auth/account` → publishes `user.deleted` to RabbitMQ
- JWT issuance (HS256, configurable TTL)
- `GET /api/auth/validate` — lightweight endpoint called by NGINX auth_request

**Does NOT own:** anything about URLs, clicks, or Redis caching.

---

### 3.2  url-service  (port 3002, internal only)

| Property | Value |
|---|---|
| Database | `pg-urls` — table: `urls` |
| Cache | Redis (cache-aside, LRU eviction) |
| Replicas | min 3, max 10 (HPA on CPU > 60%) |
| CPU | 200m request / 1000m limit |
| Memory | 128 Mi request / 512 Mi limit |

**Owns:**
- `GET /:short_code` — redirect (public)
- `POST /api/urls` — shorten (authenticated via X-User-Id header)
- `GET /api/urls` — list with cursor pagination + search
- `DELETE /api/urls/:id` — delete
- Batch click consumer (tokio::select!, size=17 OR 30s interval)
- RabbitMQ consumer: `user.deleted` → delete user's URLs + evict Redis keys
- URL expiration check on redirect (future Phase 3)

**Does NOT own:** user credentials, JWT issuance.

---

### 3.3  NGINX Ingress Controller  (API Gateway)

Not a custom Rust service. Battle-tested, Kubernetes-native, maintained by the community.

| Feature | Mechanism |
|---|---|
| TLS | cert-manager + Let's Encrypt, auto-renewed |
| Rate limiting | `nginx.ingress.kubernetes.io/limit-rps` per route |
| JWT check | `auth-url` annotation → `auth-service /api/auth/validate` |
| CORS | Annotation-based |
| Secure headers | ConfigMap server-snippet (HSTS, X-Frame-Options, CSP) |

---

### 3.4  libs/shared  (Cargo crate, compiled into both services)

```
libs/shared/src/
  config.rs     ← Settings struct, from_env()
  errors.rs     ← AppError enum implementing IntoResponse
  jwt.rs        ← Claims struct, decode/encode helpers
  middleware.rs ← extract_user_id middleware (reads X-User-Id header)
  telemetry.rs  ← OpenTelemetry + tracing subscriber init
  lib.rs
```

---

## 4. Technology Stack

| Component | Technology | Why |
|---|---|---|
| Language | Rust + Axum | Memory safe, zero-cost abstractions, best-in-class latency |
| Primary DB | PostgreSQL 17 | ACID, pg_trgm for search, battle-tested |
| Connection pooler | PgBouncer | Reduces Postgres connection overhead at scale |
| Cache | Redis + Sentinel | HA cache without Redis Cluster overhead |
| Message broker | RabbitMQ 3 (3-node) | AMQP standard, durable queues, fan-out exchanges, DLQ |
| Rust AMQP client | lapin + deadpool-lapin | Async, tokio-native, maintained |
| Ingress / Gateway | NGINX Ingress Controller | K8s-native, proven at scale, rich annotation API |
| TLS | cert-manager + Let's Encrypt | Automated issuance and renewal |
| Orchestration | Kubernetes | Industry standard, portable across all clouds |
| Local K8s | kind | Matches production API exactly, lightweight |
| Package manager | Helm 3 | Templated K8s manifests, release history, rollback |
| Metrics | Prometheus + Grafana | Pull-based, rich query language, standard dashboards |
| Log aggregation | Loki + Promtail | K8s-native, integrates with Grafana |
| Distributed tracing | OpenTelemetry → Jaeger | Vendor-neutral, correlate traces across services |
| CI pipeline | GitHub Actions | Integrated with repo, free for public repos |
| CD / GitOps | ArgoCD | Declarative, self-healing, rollback = git revert |
| Container registry | GHCR (GitHub Packages) | Free, integrated with GHA |
| Secrets | Kubernetes Secrets + Sealed Secrets | Encrypted at rest, safe to commit encrypted form |
| URL safety | Google Safe Browsing API v4 | Block phishing/malware before shortening |

---

## 5. Workspace File Structure

```
bittuly/
├── Cargo.toml                        ← workspace root
├── Cargo.lock
│
├── services/
│   ├── auth-service/
│   │   ├── Cargo.toml
│   │   ├── Dockerfile
│   │   └── src/
│   │       ├── main.rs
│   │       ├── handlers/             ← signup, login, verify_otp, delete_account, validate
│   │       ├── repository/           ← user_repo, otp_repo
│   │       ├── services/
│   │       └── events/               ← rabbitmq publisher (user.deleted)
│   │
│   └── url-service/
│       ├── Cargo.toml
│       ├── Dockerfile
│       └── src/
│           ├── main.rs
│           ├── handlers/             ← shorten, redirect, list, delete
│           ├── repository/           ← url_repo
│           ├── services/
│           ├── cache/                ← redis cache-aside logic
│           ├── consumer/             ← click batch consumer (tokio::select!)
│           └── events/               ← rabbitmq subscriber (user.deleted)
│
├── libs/
│   └── shared/
│       ├── Cargo.toml
│       └── src/
│           ├── config.rs
│           ├── errors.rs
│           ├── jwt.rs
│           ├── middleware.rs
│           ├── telemetry.rs
│           └── lib.rs
│
├── web/                              ← React frontend (unchanged)
│
├── helm/
│   ├── auth-service/                 ← Helm chart
│   ├── url-service/                  ← Helm chart
│   ├── rabbitmq/                     ← Helm chart (Bitnami)
│   ├── redis/                        ← Helm chart (Bitnami + Sentinel)
│   └── monitoring/                   ← Prometheus, Grafana, Loki, Jaeger
│
├── k8s/
│   ├── base/                         ← Namespace, RBAC, NetworkPolicy
│   └── overlays/
│       ├── local/                    ← kind cluster (self-signed TLS, low replicas)
│       └── production/               ← DOKS cluster (Let's Encrypt, full replicas)
│
├── docker/
│   ├── postgres-auth/init/           ← schema for pg-auth
│   └── postgres-urls/init/           ← schema for pg-urls
│
├── .github/
│   └── workflows/
│       ├── ci.yml                    ← PR: fmt + clippy + audit + test + build
│       └── cd.yml                    ← main: build images + push + ArgoCD sync
│
├── docker-compose.yml                ← local development only
├── TARGET.md                         ← this file
└── README.md
```

---

## 6. Database Schemas

### pg-auth

```sql
CREATE TABLE users (
    user_id    UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    username   VARCHAR(50)  NOT NULL UNIQUE,
    email      VARCHAR(255) NOT NULL UNIQUE,
    password   VARCHAR(255) NOT NULL,            -- bcrypt
    created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE TABLE otps (
    otp_id     BIGSERIAL    PRIMARY KEY,
    user_id    UUID         NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    code       VARCHAR(6)   NOT NULL,
    expires_at TIMESTAMPTZ  NOT NULL,
    used       BOOLEAN      NOT NULL DEFAULT FALSE
);
```

### pg-urls

```sql
CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE TABLE urls (
    url_id       BIGSERIAL    PRIMARY KEY,
    short_code   VARCHAR(12)  UNIQUE,
    original_url TEXT         NOT NULL,
    user_id      UUID         NOT NULL,           -- denormalized, no FK (cross-DB boundary)
    click_count  BIGINT       NOT NULL DEFAULT 0,
    expires_at   TIMESTAMPTZ,                     -- NULL = never expires
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    UNIQUE (original_url, user_id)
);

CREATE INDEX idx_urls_user_id       ON urls (user_id, url_id DESC);
CREATE INDEX idx_urls_short_code    ON urls (short_code);
CREATE INDEX idx_urls_original_trgm ON urls USING GIN (original_url gin_trgm_ops);
CREATE INDEX idx_urls_expires_at    ON urls (expires_at) WHERE expires_at IS NOT NULL;
```

> **Note on the missing foreign key:**
> In the monolith, `urls.user_id` referenced `users.user_id` via FK.
> In microservices, each service owns its own database — cross-DB foreign keys don't exist.
> Referential integrity is maintained via the RabbitMQ `user.deleted` event pattern.

---

## 7. RabbitMQ Event Bus

### Exchange topology

```
Exchange: user.events  (type: direct, durable: true)
  └── Binding: routing_key = user.deleted
        └── Queue: q.url-service.user-deleted  (durable, DLQ configured)
              └── Dead-letter: q.dlq.user-deleted

Exchange: url.events   (type: topic, durable: true)
  └── Binding: routing_key = url.*
        └── Queue: (future consumers — analytics, notifications)
```

### Event payloads

```json
// user.deleted — published by auth-service
{
  "event":     "user.deleted",
  "user_id":   "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2026-01-01T00:00:00Z"
}

// url.expired — published by url-service background sweeper (Phase 3)
{
  "event":      "url.expired",
  "short_code": "aB3xY",
  "user_id":    "550e8400-...",
  "timestamp":  "2026-01-01T00:00:00Z"
}
```

### Reliability guarantees

| Property | Value |
|---|---|
| Queue durability | `durable: true` — survive broker restart |
| Message persistence | `delivery_mode: 2` — written to disk |
| Consumer acknowledgement | Manual `ack` after successful processing |
| Retry on failure | Exponential backoff: 1s → 2s → 4s → 8s → DLQ |
| Dead-letter queue | Failed messages inspectable, replayable |

---

## 8. Kubernetes Resources

### Horizontal Pod Autoscaler — url-service
```yaml
minReplicas: 3
maxReplicas: 10
metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 60
```

### Pod Disruption Budget — url-service
```yaml
# At least 2 pods always available during node drain / rolling update
spec:
  minAvailable: 2
```

### Health probes (both services)
```yaml
livenessProbe:
  httpGet: { path: /health, port: 3002 }
  initialDelaySeconds: 10
  periodSeconds: 15
  failureThreshold: 3

readinessProbe:
  httpGet: { path: /health, port: 3002 }
  initialDelaySeconds: 5
  periodSeconds: 5
  failureThreshold: 2        # removed from load balancer after 2 failures
```

### Network Policy
```yaml
# url-service only accepts traffic from ingress controller
# url-service only talks to pg-urls and redis
# auth-service only talks to pg-auth and rabbitmq
# Nothing talks directly to databases except its owning service
```

---

## 9. Observability

### Metrics (Prometheus scrape)
Each service exposes `GET /metrics` using the `metrics` + `metrics-exporter-prometheus` crates.

| Metric | Type | Labels |
|---|---|---|
| `http_requests_total` | Counter | service, method, path, status_code |
| `http_request_duration_seconds` | Histogram | service, method, path |
| `redirect_cache_hits_total` | Counter | — |
| `redirect_cache_misses_total` | Counter | — |
| `click_batch_flush_total` | Counter | trigger (size / interval / shutdown) |
| `rabbitmq_published_total` | Counter | exchange, routing_key |
| `rabbitmq_consumed_total` | Counter | queue, result (ok / error) |
| `db_query_duration_seconds` | Histogram | service, query |
| `url_safety_checks_total` | Counter | result (safe / unsafe / error) |

### Grafana Dashboards
1. **Service Overview** — RPS, error rate, p50/p95/p99 latency per service
2. **Redirect Performance** — cache hit rate, Redis latency, DB fallback rate
3. **Infrastructure** — pod CPU/memory, HPA events, node pressure
4. **Business Metrics** — URLs created/hour, redirects/hour, top short codes by clicks

### Alerting Rules

| Alert | Condition | Severity |
|---|---|---|
| High error rate | error_rate > 1% for 5 min | Critical |
| High latency | p99 redirect > 200ms for 5 min | Warning |
| Cache hit rate degraded | cache_hit_rate < 70% for 10 min | Warning |
| Pod crash looping | restarts > 3 in 10 min | Critical |
| RabbitMQ DLQ growing | dlq_depth > 50 | Warning |
| DB connection pool exhausted | pool_wait_count > 10 | Critical |

### Distributed Tracing
- `tracing-opentelemetry` instruments every Axum handler automatically
- `traceparent` header propagated: Ingress → auth-service / url-service
- DB queries and Redis calls are child spans
- Jaeger UI for local; Tempo (Grafana Cloud) for production

### Logging
- `tracing_subscriber` in JSON format in production (`RUST_LOG=info`)
- Promtail daemonset ships all pod logs to Loki
- Log retention: 30 days
- Every log line includes `trace_id`, `service`, `pod_name`

---

## 10. Security

| Control | Implementation |
|---|---|
| TLS | cert-manager + Let's Encrypt, HTTPS-only, HTTP → HTTPS redirect |
| JWT validation | NGINX `auth_request` → `auth-service /api/auth/validate` |
| Rate limiting — redirects | 100 req/s per IP (`limit-rps: "100"`) |
| Rate limiting — shorten | 10 req/min per user (`limit-rpm: "10"` + X-User-Id key) |
| URL safety | Google Safe Browsing API v4, checked before every INSERT |
| Secrets | Kubernetes Secrets + Sealed Secrets operator (safe to git-commit) |
| Network policy | Deny-all default, allow-list per service |
| Container hardening | `runAsNonRoot`, read-only root filesystem, `allowPrivilegeEscalation: false` |
| Dependency audit | `cargo audit` in CI on every PR |
| Secure HTTP headers | HSTS, X-Frame-Options: DENY, X-Content-Type-Options, CSP |
| No internal port exposure | Only Ingress LoadBalancer is publicly accessible |

---

## 11. CI/CD Pipeline

### CI — Every Pull Request
```
1. cargo fmt --check
2. cargo clippy --workspace -- -D warnings
3. cargo audit
4. cargo test --workspace
5. docker build auth-service   (smoke-test only)
6. docker build url-service    (smoke-test only)
```

### CD — Every merge to `main`
```
1. cargo test --workspace
2. docker build auth-service  → push to GHCR with git SHA tag
3. docker build url-service   → push to GHCR with git SHA tag
4. docker build frontend      → push to GHCR with git SHA tag
5. Update helm/*/values.yaml image tags (automated commit)
6. ArgoCD detects diff → applies to K8s cluster
7. K8s rolls update with zero downtime (RollingUpdate strategy)
```

### Branch Strategy
- `main` — production-ready, protected, requires CI green + 1 review
- `dev` — integration branch for feature branches
- `feat/*` — individual features, short-lived
- Tags: `v1.0.0`, `v1.1.0` trigger GitHub Releases

---

## 12. Development Phases

### ✅ Completed (Monolith)
- User auth (signup, login, OTP, JWT)
- URL shortening (base62, BIGSERIAL, unique constraint)
- Redis cache-aside with LRU eviction
- Click batch consumer (size trigger + 30s interval flush)
- Cursor-based pagination with search (pg_trgm)
- GET /health endpoint
- System Health UI page
- Insights page (pie chart, bar chart, performance table)

---

### Phase 0 — Workspace Restructure
**Goal:** Convert monolith repo into Cargo workspace. No functional change.

- [ ] Create workspace `Cargo.toml` with `[workspace]` members
- [ ] Create `libs/shared/` crate — move JWT types, config, errors into it
- [ ] Create `services/auth-service/` skeleton — copy auth handlers/models/repos
- [ ] Create `services/url-service/` skeleton — copy url handlers/models/repos
- [ ] Both services compile independently with `shared` as a dependency
- [ ] `docker-compose.yml` builds both services, both start successfully
- [ ] All existing functionality works end-to-end

---

### Phase 1 — Service Extraction & DB Split
**Goal:** Two fully independent services with separate databases.

- [ ] `pg-auth`: users + otps tables only
- [ ] `pg-urls`: urls table only, `user_id` is a plain UUID column (no FK)
- [ ] `auth-service`: issues JWTs; validates via `/api/auth/validate`
- [ ] `url-service`: reads `X-User-Id` header injected by gateway (no JWT parsing)
- [ ] Docker Compose: add simple NGINX or Caddy as reverse proxy for local dev
- [ ] Frontend routes unchanged — still talks to `:3000`
- [ ] Full end-to-end test: signup → login → shorten → redirect → delete

---

### Phase 2 — RabbitMQ Event Bus
**Goal:** Reliable cross-service event delivery with durability guarantees.

- [ ] Add RabbitMQ to Docker Compose (management UI on :15672)
- [ ] Add `lapin` + `deadpool-lapin` to both services via shared crate
- [ ] `auth-service` publishes `user.deleted` on account deletion
- [ ] `url-service` consumes `user.deleted`:
  - Deletes all URLs for that user_id
  - Evicts their short_codes from Redis
  - Manual ack after successful DB + Redis operations
- [ ] Dead-letter queue for failed messages
- [ ] Integration test: delete account → verify URLs gone, Redis keys evicted

---

### Phase 3 — URL Expiration
**Goal:** URLs can have a TTL; expired URLs return 410 Gone.

- [ ] Add `expires_at TIMESTAMPTZ` column to pg-urls (nullable)
- [ ] `POST /api/urls` body accepts optional `expires_at` RFC3339 timestamp
- [ ] Redirect handler: if `expires_at < NOW()` → 410 Gone (check DB, skip Redis)
- [ ] Redis TTL = `min(expires_at - now(), 24h)` instead of hardcoded 24h
- [ ] Background sweeper task: every 5 minutes, delete expired URLs, publish `url.expired`
- [ ] Frontend: date picker on shorten form; expired links shown with visual indicator

---

### Phase 4 — URL Safety Check
**Goal:** Prevent shortening of phishing / malware URLs.

- [ ] Google Safe Browsing API v4 integration (free, 10k queries/day)
- [ ] Called synchronously in `shorten_url` service, before DB write
- [ ] 422 Unprocessable Entity + `{ "error": "URL flagged as unsafe" }` if matched
- [ ] Fail-open: if Safe Browsing API is unreachable, log warning + allow request
- [ ] Frontend toast for unsafe URL rejection

---

### Phase 5 — Metrics Instrumentation
**Goal:** Full Prometheus metrics from both services.

- [ ] Add `metrics` + `metrics-exporter-prometheus` crates to both services via shared
- [ ] Instrument all HTTP handlers (counter + histogram)
- [ ] Instrument cache hit/miss, batch flush trigger, RabbitMQ publish/consume
- [ ] `GET /metrics` on both services (internal port only, not exposed via ingress)
- [ ] Add Prometheus + Grafana to Docker Compose for local development
- [ ] Build the 4 dashboards listed in Section 9

---

### Phase 6 — Distributed Tracing
**Goal:** End-to-end trace correlation across services.

- [ ] Add `opentelemetry`, `tracing-opentelemetry`, `opentelemetry-otlp` to shared
- [ ] `init_telemetry()` in shared crate, called by both service `main.rs`
- [ ] Propagate `traceparent` W3C header through NGINX → services
- [ ] DB queries and Redis calls wrapped as child spans
- [ ] Jaeger in Docker Compose (UI on :16686)

---

### Phase 7 — Kubernetes Manifests (kind cluster)
**Goal:** Full system runs on local Kubernetes, identical to production structure.

- [ ] Install kind, create cluster config
- [ ] Helm chart: `auth-service` — Deployment, Service, HPA, PDB, Secret, ConfigMap
- [ ] Helm chart: `url-service` — Deployment, Service, HPA, PDB, Secret, ConfigMap
- [ ] Helm chart: RabbitMQ (Bitnami), 3-node cluster
- [ ] Helm chart: Redis (Bitnami), Sentinel mode
- [ ] Helm chart: PostgreSQL (Bitnami), two separate releases (auth + urls)
- [ ] Helm chart: monitoring stack (kube-prometheus-stack + Loki + Jaeger)
- [ ] NGINX Ingress Controller with rate-limiting annotations
- [ ] cert-manager with self-signed ClusterIssuer for local TLS
- [ ] ArgoCD installed, syncing from `helm/` directory
- [ ] NetworkPolicy: deny-all default, allow-list per service
- [ ] All health probes and PDB verified

---

### Phase 8 — CI/CD Pipeline
**Goal:** Every PR is tested; every merge to main deploys automatically.

- [ ] `.github/workflows/ci.yml`: fmt, clippy, audit, test, docker build
- [ ] `.github/workflows/cd.yml`: build images, push to GHCR, update Helm values, ArgoCD sync
- [ ] Branch protection on `main`: require CI green
- [ ] Semantic release: `git tag v1.x.x` triggers GitHub Release with changelog

---

### Phase 9 — Production Deployment (DOKS)
**Goal:** Live on a real managed Kubernetes cluster with a real domain.

- [ ] Create DigitalOcean account, provision DOKS cluster (3 × s-2vcpu-4gb)
- [ ] Configure `kubectl` with production context
- [ ] Install cert-manager + NGINX Ingress + ArgoCD on production cluster
- [ ] Configure DNS: `yourdomain.com` A record → LoadBalancer IP
- [ ] ArgoCD points to `k8s/overlays/production/`
- [ ] Deploy all Helm charts via ArgoCD
- [ ] Verify Let's Encrypt certificate issued (check cert-manager logs)
- [ ] Smoke test: shorten → redirect → Grafana shows metric
- [ ] Configure Grafana alerts → email / PagerDuty

---

### Phase 10 — Load Testing & SLO Verification
**Goal:** Verify system meets SLOs under realistic load.

- [ ] Write k6 load test scripts: redirect burst, shorten sustained, mixed
- [ ] Baseline: 1,000 RPS redirect endpoint, 5-minute sustained run
- [ ] Verify p99 latency < 15ms at peak (cache hit path)
- [ ] Verify HPA scales url-service from 3 → N pods under load
- [ ] Verify rolling deploy causes zero 5xx errors under sustained load
- [ ] Tune PgBouncer pool size, Redis connection pool, HPA thresholds based on results
- [ ] Document results in `docs/load-test-results.md`

---

## 13. SLOs

| Endpoint | p50 | p99 | Target availability |
|---|---|---|---|
| `GET /:code` (cache hit) | < 5 ms | < 15 ms | 99.9% |
| `GET /:code` (cache miss) | < 15 ms | < 60 ms | 99.9% |
| `POST /api/urls` | < 50 ms | < 200 ms | 99.5% |
| `POST /api/auth/login` | < 80 ms | < 400 ms | 99.5% |
| Overall system availability | — | — | 99.9% (monthly) |
| Redis cache hit rate | — | — | > 85% |

---

## 14. Local Development Workflow

```bash
# Start all services (Docker Compose)
docker compose up

# Run a specific service with hot-reload (requires cargo-watch)
cargo watch -x 'run --bin auth-service'
cargo watch -x 'run --bin url-service'

# Run all tests
cargo test --workspace

# Run a specific service's tests
cargo test -p auth-service

# Check everything compiles
cargo check --workspace

# Lint
cargo clippy --workspace -- -D warnings

# Security audit
cargo audit

# View structured logs with pretty-printing
docker compose logs -f url-service | jq .

# Access UIs
open http://localhost:15672    # RabbitMQ management (guest/guest)
open http://localhost:3003     # Grafana (admin/admin)
open http://localhost:16686    # Jaeger
```

---

## 15. Open Decisions

| # | Decision | Options | Status |
|---|---|---|---|
| 1 | Cloud provider for production K8s | DOKS, EKS, GKE, Hetzner | **DOKS recommended** |
| 2 | Custom short codes | Phase 3 add-on | Deferred |
| 3 | Per-day click analytics | Requires schema change | Deferred |
| 4 | Frontend deployment | Nginx in K8s vs Vercel CDN | Nginx in K8s (simpler) |
| 5 | Rate limit storage | Redis annotations vs custom middleware | NGINX annotations |

---

*Document created: 2026-06-13*
*Current status: Phase 0 — Workspace Restructure — NOT STARTED*
