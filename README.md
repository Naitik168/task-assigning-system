# Task Manager: Rust API + React frontend

A small full-stack task management app with:

- a **Rust (Axum) API**
- **email two-factor login**
- **JWT** auth
- **admin/staff role-based access**
- task assignment
- a **Redis per-user cache**
- a **React + TypeScript** frontend that consumes the API

Everything runs with one `docker compose up`.

| Layer | Technology |
|-------|------------|
| API | Rust 2021, Axum 0.8, Tokio, Serde, `validator`, `utoipa` (OpenAPI) |
| Data | PostgreSQL 16 via SQLx 0.8 (migrations in `backend/migrations`) |
| Cache | Redis 7 (`redis` crate, ConnectionManager); in-memory fallback (`moka`) |
| Security | Argon2id passwords, HMAC-SHA256 one-time codes, HS256 JWT (`jsonwebtoken`) |
| Frontend | React 19, Vite 6, TypeScript, React Router 7, Vitest + Testing Library |
| Runtime | Docker Compose: postgres, redis, backend, frontend (nginx) |

> **Step-by-step Docker walkthrough of every service:** see **[DOCKER_GUIDE.md](DOCKER_GUIDE.md)**.

---

## Quick start (Docker)

```bash
cp .env.example .env          # optional
docker compose up -d --build  # first build ~3–8 min (Rust release compile)
docker compose ps             # wait for all 4 services to be "healthy"
./scripts/validate.sh         # runs the 11-step validation flow (needs curl + jq)
```

- Frontend: http://localhost:3000
- API: http://localhost:8080 (Swagger UI at http://localhost:8080/docs)

Demo users are created by `POST /seed/users` or by the **Dev: seed** button on the login page:

| User | Email | Password | Role |
|------|-------|----------|------|
| Admin | `admin@example.com` | `Admin@12345` | admin |
| James Bond | `jamesbond@example.com` | `Bond@007007` | staff |

---

## Local setup (without Docker for the app code)

Start only the infrastructure: `docker compose up -d postgres redis`. Alternatively, point the env vars at your own Postgres and Redis.

### Backend

```bash
cd backend
cp .env.example .env
cargo run                      # applies migrations automatically (RUN_MIGRATIONS=true)
```

**Migrations.** SQL files live in `backend/migrations/` and are embedded into the binary with `sqlx::migrate!`. They run automatically on start-up. To run them manually with sqlx-cli instead:

```bash
cargo install sqlx-cli --no-default-features --features rustls,postgres
sqlx migrate run --source migrations          # uses DATABASE_URL from .env
# then start the API with RUN_MIGRATIONS=false if you prefer
```

**Seed.** Run `curl -X POST http://localhost:8080/seed/users`. It is idempotent.

### Frontend

```bash
cd frontend
npm install
npm run dev                    # http://localhost:5173, proxies /api -> http://localhost:8080
```

`VITE_API_BASE_URL` defaults to `/api`. Both the Vite dev server and the nginx container proxy `/api/*` to the API.

---

## Validation workflow

`scripts/validate.sh` performs every required step against the running API and asserts the results. It calls `POST /dev/reset` first, so it always creates **exactly 5** tasks and assigns **exactly 3**, however many times you run it.

| Step | Request |
|------|---------|
| 1 | `POST /seed/users` |
| 2 | `POST /auth/login` (Admin) returns `login_challenge_id`, **no JWT** |
| 3 | `GET /dev/email-logs/latest?email=admin@example.com` returns `verification_code` |
| 4 | `POST /auth/verify-2fa` returns Admin JWT |
| 5 | `POST /tasks` ×5 (Admin) |
| 6 | `POST /tasks/assign` `{task_ids: [3 ids], assignee_email: "jamesbond@example.com"}` |
| 7–8 | Login + 2FA as James Bond returns JWT |
| 9 | `POST /tasks` as James Bond returns **403 Forbidden** |
| 10 | `GET /tasks/view-my-tasks` returns 3 tasks, `cache.hit = false` |
| 11 | `GET /tasks/view-my-tasks` again returns `cache.hit = true` |

The same flow works in the UI. See [DOCKER_GUIDE.md §5](DOCKER_GUIDE.md#5-walking-through-the-validation-workflow) for the click-by-click version and for raw curl commands.

### Final `GET /tasks/view-my-tasks` response (James Bond)

First call, loaded from PostgreSQL:

```json
{
  "user": {
    "email": "jamesbond@example.com",
    "role": "staff"
  },
  "tasks": [
    {
      "id": "b13daa04-911b-4ade-9891-0200c743506f",
      "title": "Infiltrate SPECTRE headquarters",
      "description": "Infiltrate SPECTRE headquarters",
      "status": "todo",
      "priority": "high",
      "created_by": "admin@example.com",
      "assigned_to": "jamesbond@example.com",
      "created_at": "2026-11-17T05:47:25.429771Z",
      "updated_at": "2026-11-17T05:47:25.489215Z"
    },
    {
      "id": "6ddab472-7828-4c8a-8b33-3af0c0395cd3",
      "title": "Recover the Lektor decoder",
      "description": "Recover the Lektor decoder",
      "status": "todo",
      "priority": "medium",
      "created_by": "admin@example.com",
      "assigned_to": "jamesbond@example.com",
      "created_at": "2026-11-17T05:47:25.441392Z",
      "updated_at": "2026-11-17T05:47:25.489215Z"
    },
    {
      "id": "a34c9510-2eb2-4f12-b745-0b3fccf3c595",
      "title": "Debrief M on Operation Thunderball",
      "description": "Debrief M on Operation Thunderball",
      "status": "todo",
      "priority": "low",
      "created_by": "admin@example.com",
      "assigned_to": "jamesbond@example.com",
      "created_at": "2026-11-17T05:47:25.452666Z",
      "updated_at": "2026-11-17T05:47:25.489215Z"
    }
  ],
  "summary": {
    "total_assigned_tasks": 3
  },
  "cache": {
    "hit": false
  }
}
```

Second, identical call: the same body served from Redis, with:

```json
"cache": { "hit": true }
```

---

## Tests

| Suite | Command | Covers |
|-------|---------|--------|
| Backend unit + integration (29) | `cd backend && DATABASE_URL=postgres://taskmgr:taskmgr@localhost:5432/taskmgr cargo test` | see below |
| Backend incl. real Redis | add `TEST_REDIS_URL=redis://localhost:6379` | Redis get/set/del, hit/miss + invalidation via Redis |
| Frontend (12) | `cd frontend && npm test` | API client, login + 2FA flow, task list states, 403 message |
| In Docker | `docker compose --profile test run --rm backend-tests` / `frontend-tests` | same suites |
| End-to-end | `./scripts/validate.sh` | the 11-step flow against a running stack |
| Lint | `cargo fmt --check && cargo clippy --all-targets -- -D warnings`, `npm run typecheck` | |

Backend integration tests (`backend/tests/`) run the real router in-process. They use `tower::ServiceExt::oneshot`, and `#[sqlx::test]` gives each test an isolated, migrated database. They cover:

- seeding Admin and James Bond, idempotently, with Argon2id-hashed passwords
- login creating a challenge without returning a JWT; the code stored as an HMAC and expiring after 5 min
- the correct code returning a JWT; incorrect, expired, reused and superseded codes all rejected; lockout after 5 wrong attempts
- identical responses for wrong-password and unknown-email (no user enumeration)
- Admin creating 5 tasks and assigning exactly 3 to James Bond
- James Bond getting 403 on create, assign, list-all, update and list-users
- James Bond seeing exactly 3 tasks, ordered high → medium → low
- `view-my-tasks` returning `cache.hit` false, then true
- assignment **and** update both invalidating the affected user's cache
- validation and 404 errors returned in the uniform JSON error format

---

## Design notes

### Architecture

```
backend/src
├── main.rs            bootstrap: config → DB pool (retry) → migrations → cache → router
├── lib.rs             exposes build_router() so tests use the exact production router
├── config.rs          env-driven settings; refuses weak secrets when APP_ENV=production
├── error.rs           AppError → {"error":{"code","message"}} + HTTP status
├── extract.rs         ValidatedJson<T>: JSON + validator rules → 422 with our error body
├── models.rs          DB entities (User, Task, LoginChallenge, EmailLog) + enums
├── dto.rs             request/response contracts (API shape), separate from entities
├── middleware/auth.rs AuthUser (any valid JWT → else 401) / AdminUser (→ else 403) extractors
├── repositories/      SQL only; generic over PgExecutor (pool or transaction)
├── services/          business rules: auth, otp, jwt, password, tasks, cache, mailer, seed
└── routes/            thin HTTP handlers + utoipa annotations
frontend/src
├── api/               HTTP client (JWT attach, error mapping) + authApi / tasksApi
├── context/           AuthContext (session in sessionStorage)
├── hooks/             useAsyncData (loading / error / reload)
├── components/        LoginForm, TwoFactorForm, TaskForm, TaskTable, Alert, Spinner, ...
└── pages/             LoginPage, AdminPage, MyTasksPage
```

### Two-factor login

- `POST /auth/login` verifies the password with Argon2id, run on the blocking pool. Unknown emails are checked against a dummy hash so their timing matches a wrong password. It then creates a `login_challenges` row and "emails" a 6-digit code from the OS CSPRNG.
- **The code is never stored in plain text.** The challenge stores `HMAC-SHA256(OTP_SECRET, challenge_id ‖ code)`. Binding to the challenge id stops a hash being reused across challenges. The server secret prevents an offline brute-force of a leaked table.
- `POST /auth/verify-2fa` locks the challenge row with `SELECT … FOR UPDATE`. It then checks consumed, expired (using the DB clock) and attempt count. The digest is compared in constant time, and the challenge is marked consumed in the same transaction. Two concurrent requests therefore cannot both redeem one code.
- A new login supersedes any open challenge for that user. Five wrong codes lock the challenge.
- **Email delivery** goes through an `EmailSender` trait. The dev implementation writes to the `email_logs` table, which acts as the outbox, and logs to the console. The outbox necessarily contains the message the user would receive, so it and its read endpoint exist only in development. A real SMTP sender would implement the same trait.

### Authorisation

Handlers state their requirement in the signature: `AdminUser` for admin-only routes, `AuthUser` for any signed-in user. Because the role check is an extractor that runs before the body is parsed, a staff request to `POST /tasks` gets 403 even with an invalid body.

### Caching

`view-my-tasks` uses cache-aside, keyed per user (`taskmgr:tasks:my:{user_id}`, TTL 5 min).

- **Invalidation runs after the DB transaction commits.** Assignment invalidates the new assignee *and* any previous assignee of a reassigned task. Updates invalidate the task's current assignee.
- **Cache failures never fail a request.** A read error falls back to the database and is logged.
- **Redis is the default.** If Redis is not configured or not reachable, the API falls back to an **in-memory cache**. *Limitation:* in-memory entries are per-process, so they are lost on restart and not shared between API replicas. Run Redis when there is more than one instance.

### Error format

Every error is JSON, `{"error": {"code": "FORBIDDEN", "message": "Only admins can perform this action."}}`:

| Status | Codes |
|--------|-------|
| 400 | malformed JSON |
| 401 | `UNAUTHORIZED`, `INVALID_CREDENTIALS`, `INVALID_CODE`, `CODE_EXPIRED`, `CODE_ALREADY_USED`, `TOO_MANY_ATTEMPTS`, `INVALID_CHALLENGE` |
| 403 | `FORBIDDEN` |
| 404 | `NOT_FOUND` |
| 422 | `VALIDATION_ERROR` |
| 500 | `INTERNAL_ERROR` (details only in server logs) |

### Small route additions

The required routes are implemented as specified. These were added on top:

| Route | Purpose |
|-------|---------|
| `GET /tasks`, `GET /users?role=staff` | the admin screen's task list and assignee picker |
| `PATCH /tasks/{id}` | update, to demonstrate cache invalidation on update |
| `GET /auth/me` | session check |
| `POST /dev/reset` | repeatable validation runs |
| `GET /health`, `/docs`, `/openapi.json` | operations and API docs |

---

## Screenshots

| | |
|---|---|
| Login, step 2 (2FA code from the dev email log) | ![2FA](docs/screenshots/01-login-2fa.png) |
| Admin: 5 tasks created, 3 assigned to James Bond | ![Admin](docs/screenshots/02-admin-create-and-assign.png) |
| James Bond: 3 tasks, `cache.hit = true`, blocked from creating (403) | ![Staff](docs/screenshots/03-staff-my-tasks-cache-hit-and-403.png) |

---

## Known limitations and next steps

- Dev-only endpoints (`/seed/users`, `/dev/*`) are unauthenticated by design and disabled when `APP_ENV=production`.
- There is no rate limiting on `/auth/login` beyond per-challenge attempt limits. Production would add IP/user throttling, for example with `tower_governor`.
- JWTs are stateless with a 1-hour expiry. There are no refresh tokens and no server-side revocation list.
- The frontend stores the JWT in `sessionStorage`, which is simple and is cleared when the tab closes. An HttpOnly cookie would be more robust against XSS.
