# v2 development foundation

This document covers Stage 3 only. It is not the production deployment guide.

## Processes

- `apps/app`: React/Vite UI hosted by Tauri 2.
- `services/api`: Axum HTTP API. The client never connects directly to PostgreSQL.
- `services/migrator`: the only Stage 3 process that applies SQLx migrations.
- `services/worker`: async worker process seam. NATS JetStream is wired in Stage 6.
- PostgreSQL 18 is the v2 system of record.

## Local dependencies

```sh
docker compose -f infra/compose.dev.yml up -d
```

Example local environment:

```text
DATABASE_URL=postgres://school_collect:school_collect_local_only@127.0.0.1:5432/school_collect
APP_BIND_ADDR=127.0.0.1:3000
APP_CORS_ORIGIN=http://127.0.0.1:1420
VITE_API_BASE_URL=http://127.0.0.1:3000
```

Do not use the local development password outside a loopback-only development machine.

## Database roles

The runtime API must not own schema migrations. Stage 6 introduces separate production runtime/migration roles and least-privilege grants. The separate migrator binary establishes that process boundary now.

## Health

- `GET /health`: process liveness; does not require PostgreSQL.
- `GET /ready`: dependency readiness; returns 503 until PostgreSQL can answer a readiness query.
- `GET /openapi.json`: generated OpenAPI document.

## Commands

```sh
pnpm install
pnpm --filter @school-collect/app dev
cargo run -p school-collect-migrator
cargo run -p school-collect-api
pnpm --filter @school-collect/app tauri dev
```

Stage 5 replaces the diagnostic UI with the approved Figma design-system implementation.
