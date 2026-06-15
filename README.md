# Bittuly - Distributed URL Shortener

Bittuly is a production-grade, distributed URL shortener built with Rust (Axum) and React (Vite). It uses a microservices architecture, separating authentication and URL management into distinct services with their own isolated databases.

## 🏗️ System Architecture

- **`auth-service` (Port 3001)**: Handles user signups, stateless OTP verification (via email or console), JWT generation, and user management.
- **`url-service` (Port 3002)**: Handles URL shortening, redirects, and click analytics tracking. Uses Redis for caching short URLs.
- **`consumer-service`**: Background asynchronous worker that consumes events from RabbitMQ (e.g., batch-updating clicks, cascading user deletion cleanups).
- **`libs/shared`**: Shared Rust crate containing RabbitMQ configurations, JWT logic, database connections, and middleware.
- **`Event Bus (RabbitMQ)`**: Core messaging backbone providing *At-Least-Once* delivery for reliable, decoupled inter-service communication.
- **`web/` (Port 5173)**: Modern React frontend built with Vite, Tailwind CSS, and a beautiful Notion-inspired design system.

---

## 📨 Event Bus Workflows (RabbitMQ)

To ensure the microservices remain decoupled, highly performant, and reliable, we use RabbitMQ as the central messaging backbone.

### 1. User Deletion Workflow (`user_deleted_queue`)
When a user deletes their account:
1. **Publisher (`auth-service`)**: Deletes the user from `pg-auth` and publishes a `UserDeletedEvent(user_id)` to RabbitMQ. It immediately returns `204 No Content` to the user.
2. **Consumer (`consumer-service`)**: Asynchronously listens for the event.
3. **Execution**: The consumer connects to `pg-urls` to delete all links belonging to the `user_id` using a `RETURNING short_code` query. It then loops through those codes and evicts them from the **Redis Cache**.
4. **Resilience**: If the `pg-urls` database is temporarily down, the consumer utilizes **Native Exponential Backoff**. It publishes the message to tiered delay queues (3s, 9s, 27s) using RabbitMQ's Time-To-Live (TTL) feature. If it fails 4 times, it is routed to a permanent Dead Letter Queue (`user_deleted_dlq`).

### 2. Analytics Tracking Workflow (`click_events_queue`)
When a user clicks a shortened URL:
1. **Publisher (`url-service`)**: Responds with a fast `302 Redirect` (from Redis) and immediately fires a `short_code` payload into the RabbitMQ `click_events_queue` in the background. The user never waits for the database.
2. **Consumer (`consumer-service`)**: Pulls the click events off the queue and holds them in an in-memory `HashMap`.
3. **Execution (Batching)**: To prevent hammering the database, the consumer flushes the clicks to Postgres in **bulk** either every 30 seconds (Timer-based) or every 17 clicks (Size-based). It executes a single atomic `UPDATE` using `unnest` arrays.
4. **Resilience**: If the database is offline during a flush, the consumer sleeps its thread (Dynamic Backoff: 3s, 9s, 27s) and `NACK`s the batch, cleanly applying backpressure to the RabbitMQ queue until the DB recovers.

---

## 🚀 Getting Started

### 1. Start the Infrastructure (Databases & Cache)
The system requires two separate PostgreSQL databases and a Redis cache. These are fully containerized.

```bash
docker compose up -d
```
This spins up:
- `postgres-auth` (Port 5432) — Database: `bittuly_auth`
- `postgres-urls` (Port 5433) — Database: `bittuly_urls`
- `redis` (Port 6379)
- `rabbitmq` (Port 5672) and Management UI (Port 15672)

*Note: The Postgres schemas are automatically created via the init scripts in `docker/postgres-auth/init` and `docker/postgres-urls/init` the first time the containers start.*

### 2. Configure Environment Variables
**Backend (`/.env`):**
The root `.env` file configures the backend.
By default, `MODE=development` will bypass real SMTP emails and print your OTP code to the terminal.
To test real emails, set `MODE=production` and ensure `SMTP_USER` and `SMTP_PASS` are configured correctly.

**Frontend (`web/.env`):**
Ensure the frontend `.env` points to the correct backend ports:
```env
VITE_AUTH_API_URL=http://localhost:3001
VITE_URLS_API_URL=http://localhost:3002
```

### 3. Start the Microservices
We have set up convenient Cargo aliases using `cargo-watch` so that all services auto-reload on code changes. You will need three separate terminal windows for the backend.

*(Ensure you have cargo-watch installed: `cargo install cargo-watch`)*

**Terminal 1 (Auth Service):**
```bash
cargo dev-auth
```

**Terminal 2 (URL Service):**
```bash
cargo dev-urls
```

**Terminal 3 (Consumer Service):**
```bash
cargo dev-consumer
```

### 4. Start the Frontend
Open a third terminal window, navigate to the `web/` directory, install dependencies, and start the Vite development server:

```bash
cd web
npm install
npm run dev
```

Your frontend is now available at `http://localhost:5173`!

---

## 🛠️ Useful Commands

### Wiping the Databases
If you modify the SQL schema files and need to start fresh, you must destroy the Docker volumes and recreate them:
```bash
docker compose down -v
docker compose up -d
```

### Building for Production
```bash
cargo build --release
```
This builds optimized binaries for both services in the `target/release` folder.

---

## 🛡️ Continuous Integration (CI) & Git Hooks

This project uses GitHub Actions to automatically format, lint, and test both the Rust backend and React frontend on every push.

If you want to run these exact same checks locally before pushing, you can run the provided check script:
```bash
./scripts/check.sh
```

**Recommended: Set up Git Hooks**

To ensure your code is always formatted and passes checks before pushing, we recommend setting up two git hooks:

**1. Pre-Commit Hook (Auto-formatting)**
This hook automatically runs `cargo fmt` to format your Rust code right before creating a commit.
```bash
echo -e '#!/bin/sh\n\necho "==> Formatting Rust code (cargo fmt)..."\ncargo fmt --workspace\n\necho "==> Rust formatting complete."\n' > .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

**2. Pre-Push Hook (CI Checks)**
This hook runs the full check script before pushing code to GitHub. If tests or linting fail, the push is aborted.
```bash
cp scripts/check.sh .git/hooks/pre-push
chmod +x .git/hooks/pre-push
```
