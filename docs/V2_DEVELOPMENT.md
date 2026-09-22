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

`/health` answers from the process alone, so it stays 200 while a dependency is down. `/ready` runs a real query against PostgreSQL on every request, so it reports current dependency state rather than a value cached at startup.

Missing configuration is not a degraded runtime mode. The API refuses to start without `DATABASE_URL`. The pool connects lazily, so a database that is merely unreachable is a readiness failure instead of a startup failure.

## Lockfiles

`pnpm-lock.yaml` and `Cargo.lock` are committed and are the source of truth. CI installs with `pnpm install --frozen-lockfile` and runs cargo with `--locked`, so an uncommitted dependency change fails the build instead of silently resolving a different version.

## Configuration and secret boundary

Every value the server reads comes from the environment at startup: `DATABASE_URL`, `APP_BIND_ADDR`, `APP_CORS_ORIGIN`. None are committed, and the repository contains no default credential for any non-loopback target.

The client is untrusted, so it receives no secret of any kind. `apps/app` reads only `VITE_API_BASE_URL`, a public endpoint address. Anything prefixed `VITE_` is compiled into the shipped bundle and must be treated as public. Server credentials must never be exposed through a `VITE_` variable.

Known Stage 3 limitation: `tauri.conf.json` currently ships one CSP whose `connect-src` allows `http://127.0.0.1:*`. That is a development convenience, and a production build must not keep loopback HTTP in `connect-src`. Stage 6 splits development and production configuration and replaces this with the real API origin over HTTPS.

The local development password in `infra/compose.dev.yml` is for loopback-only containers. It is not a shared secret and must not be reused for any reachable database.

## Commands

```sh
pnpm install
pnpm --filter @school-collect/app dev
cargo run -p school-collect-migrator
cargo run -p school-collect-api
pnpm --filter @school-collect/app tauri dev
```

Stage 5 replaces the diagnostic UI with the approved Figma design-system implementation.

## Mobile targets: unverified

Mobile is not a Stage 3 deliverable and is not verified.

What was checked: the pinned `tauri-cli` 2.11.5 exposes `tauri android init` and `tauri ios init`, so the CLI surface exists. Nothing beyond that was confirmed.

What was not checked, and why: the development machine has no Android SDK/NDK (`ANDROID_HOME` and NDK are unset) and only the `x86_64-pc-windows-msvc` Rust target installed, so no Android target could be built. iOS additionally requires macOS, which is unavailable here. `android init` and `ios init` were not run, because generating mobile project scaffolding would add unverifiable files to this PR.

Consequence: treat Android/iOS target and plugin compatibility as an open Stage 6+ decision. Do not record it as supported.
