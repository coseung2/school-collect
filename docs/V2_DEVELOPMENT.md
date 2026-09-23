# v2 development foundation

This document covers the development foundation and Stage 6 configuration checks. It is not the production deployment guide.

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
APP_ENV=development
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

Stage 6 now validates the server environment boundary:

- `APP_ENV` is required and accepts only `development`, `staging`, or `production`; omission never silently selects development.
- Development may use the loopback CORS default; staging and production require `APP_CORS_ORIGIN`.
- Staging and production require `OIDC_ISSUER_URL` with an HTTPS scheme and `OIDC_AUDIENCE`.
- The client still receives only the public API origin. OIDC secrets and verification material remain server-side.

The local development password in `infra/compose.dev.yml` is for loopback-only containers. It is not a shared secret and must not be reused for any reachable database.

## Commands

```sh
pnpm install
pnpm --filter @school-collect/app dev
cargo run -p school-collect-migrator
cargo run -p school-collect-api
pnpm --filter @school-collect/app tauri dev
```

Stage 5 is replacing the diagnostic UI with the approved Figma design-system
implementation. The current code migration is recorded in
`docs/STAGE5_CODE_MIGRATION.md`; it adds no production data or database seed.

## Windows native development

Use PowerShell 7: `./scripts/windows-dev.ps1 -Action build` (or `test`, `check`, `dev`).
The script selects rustup's pinned MSVC toolchain and the installed Visual Studio C++ tools without changing the system PATH.
On 2026-09-23 the release build and process/window launch succeeded, as did full workspace tests and clippy with warnings denied.
The previous GNU linker failures came from PATH selecting a separate GNU installation; MSVC was already installed.

## Mobile targets: partially verified

Mobile is not a Stage 3 deliverable and is not verified.

On 2026-09-23 Android SDK 36 and NDK 28.2 were found locally, the ARM64 Rust target was installed, and the Tauri ARM64 release library compiled successfully.

APK packaging failed when Tauri attempted a symbolic link: this Windows session lacks that privilege. The generated Android project remains local pending packaging validation. No APK, device interaction, or mobile plugin compatibility is certified. iOS requires macOS and remains unverified.

Consequence: treat Android/iOS target and plugin compatibility as an open Stage 6+ decision. Do not record it as supported.
