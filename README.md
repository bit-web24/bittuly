# Bittuly - Distributed URL Shortener

Bittuly is a production-grade, distributed URL shortener built with Rust (Axum) and React (Vite). It uses a microservices architecture, separating authentication and URL management into distinct services with their own isolated databases.

## 🏗️ System Architecture

- **`auth-service` (Port 3001)**: Handles user signups, stateless OTP verification (via email or console), JWT generation, and user management.
- **`url-service` (Port 3002)**: Handles URL shortening, redirects, and click analytics tracking. Uses Redis for caching short URLs and a background async worker for processing click metrics.
- **`libs/shared`**: Shared Rust crate containing JWT logic, configurations, database connections, and middleware.
- **`web/` (Port 5173)**: Modern React frontend built with Vite, Tailwind CSS, and a beautiful Notion-inspired design system.

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
We have set up convenient Cargo aliases using `cargo-watch` so that both services auto-reload on code changes. You will need two separate terminal windows for the backend.

*(Ensure you have cargo-watch installed: `cargo install cargo-watch`)*

**Terminal 1 (Auth Service):**
```bash
cargo dev-auth
```

**Terminal 2 (URL Service):**
```bash
cargo dev-urls
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

**Recommended: Set up a Git Pre-Push Hook**
To ensure you never push broken code to GitHub, you can force Git to automatically run this script every time you type `git push`. If the checks fail, the push is aborted.

To install the hook, simply run:
```bash
cp scripts/check.sh .git/hooks/pre-push
chmod +x .git/hooks/pre-push
```
