# Docker Guide: Running and Exploring Every Service

This guide brings up the whole system with Docker Compose. It then walks through each container: what it does, how to check it's healthy, and how to look inside it. It finishes with the full validation workflow and day-to-day operations.

---

## 1. Architecture

```
                         ┌────────────────────────────── docker network: task-manager_default ─────────────────────────────┐
                         │                                                                                                   │
  Browser ──► :3000 ───► │  frontend (nginx)                backend (Rust / Axum)              postgres (16-alpine)          │
                         │  • serves React build   /api/* ► • REST API :8080          ───────► • users, tasks,              │
  curl/Postman ► :8080 ─►│  • proxies /api → backend        • JWT + email 2FA                    login_challenges,          │
                         │  • SPA fallback                  • runs migrations on boot            email_logs                 │
                         │                                  • per-user task cache    ───────► redis (7-alpine)              │
                         │                                                                      • taskmgr:tasks:my:{id}    │
                         └───────────────────────────────────────────────────────────────────────────────────────────────────┘
                                                              volumes: postgres-data, redis-data
```

| Service    | Image / build                    | Host port | Container port | Health check                 | Persistent data |
|------------|----------------------------------|-----------|----------------|------------------------------|-----------------|
| `postgres` | `postgres:16-alpine`             | 5432      | 5432           | `pg_isready`                 | `postgres-data` |
| `redis`    | `redis:7-alpine` (AOF enabled)   | 6379      | 6379           | `redis-cli ping`             | `redis-data`    |
| `backend`  | `./backend/Dockerfile` (runtime) | 8080      | 8080           | `curl /health`               | –               |
| `frontend` | `./frontend/Dockerfile` (nginx)  | 3000      | 80             | `wget /healthz`              | –               |

**Start-up order** is enforced with `depends_on: condition: service_healthy`:
`postgres` + `redis` → `backend` (applies migrations, connects to Redis) → `frontend`.

**Test-only services** (profile `test`, never started by `docker compose up`): `backend-tests`, `frontend-tests`.

---

## 2. Prerequisites

| Tool | Version | Check |
|------|---------|-------|
| Docker Engine / Docker Desktop | 24+ | `docker --version` |
| Docker Compose plugin | v2.20+ | `docker compose version` |
| curl + jq (for the validation script only) | any | `jq --version` |

You also need ports **3000, 8080, 5432 and 6379** free. You can change them in `.env` (see section 9).

> The first build compiles the Rust API in release mode, which takes about **3–8 minutes** depending on your machine. Later builds reuse the cached dependency layer and take well under a minute.

---

## 3. Quick start

```bash
cd task-manager
cp .env.example .env            # optional: every value has a default
docker compose up -d --build    # build images and start all 4 services
docker compose ps               # wait until every service shows "(healthy)"
```

Expected `docker compose ps` output (abridged):

```
NAME                       SERVICE    STATUS                   PORTS
task-manager-postgres-1    postgres   Up 30 seconds (healthy)  0.0.0.0:5432->5432/tcp
task-manager-redis-1       redis      Up 30 seconds (healthy)  0.0.0.0:6379->6379/tcp
task-manager-backend-1     backend    Up 20 seconds (healthy)  0.0.0.0:8080->8080/tcp
task-manager-frontend-1    frontend   Up 5 seconds (healthy)   0.0.0.0:3000->80/tcp
```

Then open:

| What | URL |
|------|-----|
| Frontend | http://localhost:3000 |
| API health | http://localhost:8080/health |
| Swagger UI | http://localhost:8080/docs |
| OpenAPI JSON | http://localhost:8080/openapi.json |

Run the whole 11-step validation in one command:

```bash
./scripts/validate.sh                                   # talks to the API on :8080
API_URL=http://localhost:3000/api ./scripts/validate.sh # same, through the nginx proxy
```

A `Makefile` wraps the common commands: `make up`, `make ps`, `make logs`, `make validate`, `make test`, `make down` and `make clean`.

---

## 4. Tour of each service

### 4.1 `postgres`: the database

Stores users, tasks, 2FA challenges and the development email outbox. The API applies the schema on start-up from the migrations embedded in its binary.

```bash
# Open a psql shell
docker compose exec postgres psql -U taskmgr -d taskmgr

# Useful queries (run inside psql, or pass with -c "...")
\dt                                                      -- tables (incl. _sqlx_migrations)
SELECT version, description, success FROM _sqlx_migrations;
SELECT email, role, left(hashed_password, 30) AS hash FROM users;      -- Argon2id hashes
SELECT t.title, t.priority, t.status, u.email AS assignee
  FROM tasks t LEFT JOIN users u ON u.id = t.assigned_to_id ORDER BY t.created_at;
SELECT id, left(code_hash, 16) AS code_hash, expires_at, consumed_at, failed_attempts
  FROM login_challenges ORDER BY created_at DESC LIMIT 5;    -- codes stored as HMAC only
SELECT to_email, body, created_at FROM email_logs ORDER BY created_at DESC LIMIT 5;
```

Health: `docker compose exec postgres pg_isready -U taskmgr`

### 4.2 `redis`: the per-user task cache

`GET /tasks/view-my-tasks` uses the cache-aside pattern. The key is `taskmgr:tasks:my:{user_id}` and the TTL is `CACHE_TTL_SECONDS`, 300 seconds by default. A key is deleted whenever a task assigned to that user is assigned or updated.

```bash
docker compose exec redis redis-cli ping                       # PONG
docker compose exec redis redis-cli --scan --pattern 'taskmgr:*'
docker compose exec redis redis-cli TTL taskmgr:tasks:my:<user-id>
docker compose exec redis redis-cli GET taskmgr:tasks:my:<user-id> | jq .

# Watch cache traffic live while you click around the UI:
docker compose exec redis redis-cli MONITOR
#   "GET" "taskmgr:tasks:my:…"                  <- every view-my-tasks call
#   "SET" "taskmgr:tasks:my:…" … "EX" "300"     <- on a cache miss
#   "DEL" "taskmgr:tasks:my:…"                  <- on assign / update (invalidation)
```

If Redis is unreachable at start-up, the API logs a warning and falls back to an in-memory cache. `GET /health` reports `"cache_backend": "redis"` or `"in-memory"`.

### 4.3 `backend`: the Rust API

An Axum + SQLx + Tokio service built in a multi-stage Dockerfile. The runtime image is `debian:bookworm-slim` with a single binary that runs as a non-root user.

```bash
curl -s localhost:8080/health | jq
# {"status":"ok","database":"ok","cache_backend":"redis","cache":"ok"}

docker compose logs -f backend                       # structured request logs
docker compose logs backend | grep dev-email         # 2FA codes ("emails") appear here
```

What happens at start-up (visible in the logs):

1. It connects to Postgres, retrying for up to 30 s.
2. It applies migrations: `database migrations applied`.
3. It connects to Redis: `cache backend: redis`.
4. It starts listening: `listening on http://0.0.0.0:8080`.

**Endpoints** (also browsable in Swagger at `/docs`):

| Method | Path | Auth | Purpose |
|--------|------|------|---------|
| GET  | `/health` | – | Liveness/readiness |
| POST | `/seed/users` | – (dev) | Create Admin + James Bond (idempotent) |
| POST | `/auth/login` | – | Password check → 2FA challenge (no JWT) |
| GET  | `/dev/email-logs/latest?email=` | – (dev) | Read the latest emailed code |
| POST | `/auth/verify-2fa` | – | Code → JWT |
| GET  | `/auth/me` | any | Current user |
| GET  | `/users?role=staff` | admin | Assignee list |
| POST | `/tasks` | admin | Create task (staff → **403**) |
| GET  | `/tasks` | admin | List all tasks |
| POST | `/tasks/assign` | admin | Assign tasks to a staff user |
| PATCH| `/tasks/{id}` | admin | Update task (invalidates cache) |
| GET  | `/tasks/view-my-tasks` | any | My tasks + `cache.hit` |
| POST | `/dev/reset` | – (dev) | Delete tasks/challenges/emails, clear cache |

"(dev)" endpoints exist only while `APP_ENV=development`.

### 4.4 `frontend`: the React app behind nginx

A multi-stage build: Node 22 compiles the Vite/React/TypeScript app, then `nginx:1.27-alpine` serves the static files and **reverse-proxies `/api/*` to `backend:8080`**. The browser therefore only ever talks to `localhost:3000`, so no CORS is involved in Docker.

```bash
curl -s localhost:3000/healthz                 # ok
curl -s localhost:3000/api/health | jq         # proxied to the backend
docker compose exec frontend cat /etc/nginx/conf.d/default.conf   # rendered config
docker compose logs -f frontend                # nginx access log
```

The proxy target comes from `BACKEND_URL` (default `http://backend:8080`). It is substituted into `nginx/default.conf.template` when the container starts.

---

## 5. Walking through the validation workflow

### 5.1 In the browser (http://localhost:3000)

| # | Action | Expected |
|---|--------|----------|
| 1 | Login page → **Dev: seed Admin & James Bond** | "Seeded users: admin@example.com (admin…), jamesbond@example.com (staff…)" |
| 2 | Click **Admin** under demo accounts → **Continue** | Step 2 screen. No JWT issued yet. |
| 3 | **Dev: fetch code from email log** (or read it from `docker compose logs backend`) | The 6-digit code is filled in |
| 4 | **Verify & sign in** | You land on the Admin screen |
| 5 | Create 5 tasks with the form | "All tasks (5)" |
| 6 | Tick 3 tasks → Assign to James Bond → **Assign selected** | "Assigned 3 task(s) to jamesbond@example.com." |
| 7–8 | **Sign out** → **James Bond** → Continue → fetch code → Verify | You land on "My assigned tasks" |
| 9 | "Create a task" card → **Try to create task** | Yellow **Access denied (403 Forbidden)** message |
| 10 | My assigned tasks | Exactly 3 tasks, badge `cache.hit = false (loaded from database)` |
| 11 | **Refresh** | Badge `cache.hit = true (served from cache)` |

Demo credentials: `admin@example.com / Admin@12345` and `jamesbond@example.com / Bond@007007`. They come from `SEED_ADMIN_PASSWORD` and `SEED_STAFF_PASSWORD`.

### 5.2 With curl, step by step

```bash
API=http://localhost:8080
curl -s -X POST $API/dev/reset | jq                     # optional: clean slate

# 1. Seed users
curl -s -X POST $API/seed/users | jq

# 2. Admin login -> challenge (no token)
ADMIN_CH=$(curl -s -X POST $API/auth/login -H 'Content-Type: application/json' \
  -d '{"email":"admin@example.com","password":"Admin@12345"}' | jq -r .login_challenge_id)

# 3. Read the code from the dev email log
CODE=$(curl -s "$API/dev/email-logs/latest?email=admin@example.com" | jq -r .verification_code)

# 4. Verify -> JWT
ADMIN=$(curl -s -X POST $API/auth/verify-2fa -H 'Content-Type: application/json' \
  -d "{\"login_challenge_id\":\"$ADMIN_CH\",\"code\":\"$CODE\"}" | jq -r .access_token)

# 5. Create 5 tasks
for p in high medium low medium low; do
  curl -s -X POST $API/tasks -H "Authorization: Bearer $ADMIN" -H 'Content-Type: application/json' \
    -d "{\"title\":\"Mission ($p)\",\"priority\":\"$p\"}" | jq -r '.id + "  " + .title'
done

# 6. Assign the first 3 to James Bond
IDS=$(curl -s $API/tasks -H "Authorization: Bearer $ADMIN" | jq -c '[.[0:3][].id]')
curl -s -X POST $API/tasks/assign -H "Authorization: Bearer $ADMIN" -H 'Content-Type: application/json' \
  -d "{\"task_ids\":$IDS,\"assignee_email\":\"jamesbond@example.com\"}" | jq '.assigned_count'

# 7-8. James Bond login + 2FA
BOND_CH=$(curl -s -X POST $API/auth/login -H 'Content-Type: application/json' \
  -d '{"email":"jamesbond@example.com","password":"Bond@007007"}' | jq -r .login_challenge_id)
CODE=$(curl -s "$API/dev/email-logs/latest?email=jamesbond@example.com" | jq -r .verification_code)
BOND=$(curl -s -X POST $API/auth/verify-2fa -H 'Content-Type: application/json' \
  -d "{\"login_challenge_id\":\"$BOND_CH\",\"code\":\"$CODE\"}" | jq -r .access_token)

# 9. James Bond cannot create tasks -> 403
curl -s -o /dev/null -w '%{http_code}\n' -X POST $API/tasks -H "Authorization: Bearer $BOND" \
  -H 'Content-Type: application/json' -d '{"title":"nope"}'

# 10-11. View my tasks twice: cache.hit false, then true
curl -s $API/tasks/view-my-tasks -H "Authorization: Bearer $BOND" | jq '{summary, cache}'
curl -s $API/tasks/view-my-tasks -H "Authorization: Bearer $BOND" | jq '{summary, cache}'
```

**Try the 2FA edge cases:**

```bash
# Reuse the same code -> 401 CODE_ALREADY_USED
curl -s -X POST $API/auth/verify-2fa -H 'Content-Type: application/json' \
  -d "{\"login_challenge_id\":\"$BOND_CH\",\"code\":\"$CODE\"}" | jq .error

# Wrong code -> 401 INVALID_CODE (after 5 wrong attempts: TOO_MANY_ATTEMPTS)
# Expired code -> 401 CODE_EXPIRED. To simulate expiry without waiting:
docker compose exec postgres psql -U taskmgr -d taskmgr \
  -c "UPDATE login_challenges SET expires_at = now() - interval '1 second' WHERE consumed_at IS NULL;"
```

**See cache invalidation in action:**

```bash
TASK=$(echo $IDS | jq -r '.[0]')
curl -s -X PATCH $API/tasks/$TASK -H "Authorization: Bearer $ADMIN" -H 'Content-Type: application/json' \
  -d '{"status":"in_progress"}' | jq .status
curl -s $API/tasks/view-my-tasks -H "Authorization: Bearer $BOND" | jq .cache   # {"hit": false}
```

### 5.3 With Swagger UI

Open http://localhost:8080/docs. Call `/auth/login`, then `/dev/email-logs/latest`, then `/auth/verify-2fa`. Copy `access_token`, click **Authorize** and paste the token. You can now call the protected endpoints.

---

## 6. Running the tests in Docker

```bash
docker compose up -d postgres redis                         # test DBs are created on this server
docker compose --profile test run --rm backend-tests        # 29 Rust tests (unit + integration + Redis)
docker compose --profile test run --rm frontend-tests       # 12 Vitest/Testing Library tests
```

The backend integration tests use `#[sqlx::test]`. It creates a throwaway database for each test on the `postgres` container and drops it afterwards, so tests never touch the application's data.

---

## 7. Day-to-day operations

| Task | Command |
|------|---------|
| Follow all logs | `docker compose logs -f` |
| Logs for one service | `docker compose logs -f backend` |
| Rebuild + restart one service after code changes | `docker compose up -d --build backend` |
| Restart without rebuilding | `docker compose restart backend` |
| Shell inside a container | `docker compose exec backend sh` / `exec postgres sh` |
| Stop everything (keep data) | `docker compose down` |
| Reset app data but keep users | `curl -X POST localhost:8080/dev/reset` |
| Wipe everything incl. volumes | `docker compose down -v` |
| Resource usage | `docker stats` |

---

## 8. Hybrid mode: infrastructure in Docker, code on your machine

This is useful while you develop, because it gives hot reload for the UI and quick `cargo run` cycles.

```bash
docker compose up -d postgres redis

# API
cd backend && cp .env.example .env && cargo run          # http://localhost:8080

# UI (new terminal); Vite proxies /api -> localhost:8080
cd frontend && npm install && npm run dev                # http://localhost:5173
```

---

## 9. Configuration

Compose reads `.env` from the project root. The most useful variables:

| Variable | Default | Notes |
|----------|---------|-------|
| `APP_ENV` | `development` | `production` disables `/seed`, `/dev/*` and requires strong secrets |
| `FRONTEND_PORT` / `BACKEND_PORT` | 3000 / 8080 | Host ports |
| `POSTGRES_PORT` / `REDIS_PORT` | 5432 / 6379 | Change if you already run Postgres/Redis locally |
| `JWT_SECRET` / `OTP_SECRET` | dev values | `openssl rand -hex 32` for anything real |
| `JWT_TTL_MINUTES` | 60 | Token lifetime |
| `OTP_TTL_SECONDS` | 300 | 2FA code lifetime (5 min) |
| `OTP_MAX_ATTEMPTS` | 5 | Wrong codes before a challenge locks |
| `CACHE_TTL_SECONDS` | 300 | Safety-net expiry for cached task lists |
| `SEED_ADMIN_PASSWORD` / `SEED_STAFF_PASSWORD` | `Admin@12345` / `Bond@007007` | Used by `/seed/users` |

If you change `SEED_*` after the users already exist, either run `docker compose down -v` or delete them from `users`. Seeding never overwrites existing passwords.

---

## 10. Troubleshooting

| Symptom | Cause / fix |
|---------|-------------|
| `Bind for 0.0.0.0:5432 failed: port is already allocated` | A local Postgres or Redis is running. Set `POSTGRES_PORT=5433` and/or `REDIS_PORT=6380` in `.env`. |
| `backend` stays `(health: starting)` or restarts | `docker compose logs backend`. Usually the DB credentials in `.env` changed after the volume was created. Run `docker compose down -v` to recreate it. |
| `frontend` never starts | It waits for `backend` to be healthy. Fix the backend first. |
| UI shows "Cannot reach the API" | Check `curl localhost:3000/api/health`. If that fails, check `docker compose logs frontend backend`. |
| Login says "Invalid email or password." | Users weren't seeded. Click **Dev: seed** or `curl -X POST localhost:8080/seed/users`. |
| `/seed/users` returns 404 | `APP_ENV=production`. Dev endpoints are disabled. |
| `/health` shows `"cache_backend":"in-memory"` | Redis was unreachable when the API started. Run `docker compose restart backend` once Redis is healthy. |
| `validate.sh: jq is required` | Install jq (`brew install jq`, `apt install jq`, or `winget install jqlang.jq`). On Windows use Git Bash or WSL. |
| `validate.sh: bad interpreter` on Windows | Line endings got converted. Run `git config core.autocrlf input` and re-checkout, or `dos2unix scripts/validate.sh`. |
| First build is slow | Expected: the Rust release build. Later builds are cached. |
| Apple Silicon / ARM | All base images are multi-arch, so it builds natively. |
