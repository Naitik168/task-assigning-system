# syntax=docker/dockerfile:1

# ---------- builder: compiles the API (dependencies cached in their own layer) ----------
FROM rust:1-slim-bookworm AS builder
WORKDIR /app

# 1) Build dependencies only, using placeholder sources. This layer is reused
#    as long as Cargo.toml / Cargo.lock don't change.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
    && echo "fn main() {}" > src/main.rs \
    && touch src/lib.rs \
    && cargo build --release --locked \
    && rm -rf src

# 2) Build the real application. Migrations are embedded into the binary
#    at compile time by `sqlx::migrate!`, so the runtime image needs no SQL files.
COPY src ./src
COPY migrations ./migrations
COPY tests ./tests
RUN touch src/main.rs src/lib.rs && cargo build --release --locked

# ---------- test: `docker compose --profile test run --rm backend-tests` ----------
FROM builder AS test
CMD ["cargo", "test", "--release", "--locked"]

# ---------- runtime: small image with just the binary ----------
FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --no-create-home app

COPY --from=builder /app/target/release/task-manager-api /usr/local/bin/task-manager-api

USER app
ENV BIND_ADDR=0.0.0.0:8080 \
    RUST_LOG=task_manager_api=info,tower_http=info
EXPOSE 8080

HEALTHCHECK --interval=10s --timeout=3s --start-period=20s --retries=5 \
    CMD curl -fsS http://localhost:8080/health || exit 1

CMD ["task-manager-api"]
