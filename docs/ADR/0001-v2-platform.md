# ADR-0001: Desktop-first, API-first v2 platform

Date: 2026-09-21
Status: accepted direction; implementation and infrastructure pending

## Context

The user chose Tauri desktop first with future mobile support, Figma for design, a develop integration branch, and a full rewrite. The former Supabase project was discarded. Preserve useful requirements, not its deployed infrastructure assumptions.

## Decision

Use Tauri 2 + React/Vite and a separately deployed Rust/Axum API backed by PostgreSQL/SQLx. ZITADEL handles identity through OIDC/PKCE; School Collect owns school membership and authorization. Use private R2 for objects and Cloudflare as edge, not as an authorization substitute. Use NATS JetStream with a PostgreSQL transactional outbox and idempotent workers. SQLite is limited to client drafts/cache/outbox, not the system of record.

Keep the API a modular monolith, separating worker execution and identity infrastructure. Do not add Kubernetes, service mesh or independent services per module without measured operational requirements. Self-hosting is an available deployment choice, not intrinsically more secure or more available than managed infrastructure.

Preserve server-authoritative domain invariants. Client validation improves UX; it does not grant permission. Generate client contracts from OpenAPI. Do not share server secrets or administration dependencies into Tauri crates.

## Consequences

This stack introduces Rust, identity lifecycle, database backups, queue semantics, native signing and platform-specific validation. Each is a delivery gate, not a benefit obtained just by selecting a product. Single-node Compose is not HA. Tenant isolation requires server checks, database policy/roles and negative tests. Outbox and consumer idempotency are required despite broker delivery guarantees.

Production hosting location, domain, resource budget, data handling policy, identity bootstrap, R2 account, platform signing credentials and Figma team remain explicit external inputs. No paid resource creation is implied by this ADR.

## Delivery

Follow `../V2_PLAN.md`. Foundation PRs prepare governance first. Actual workspace/API/native implementation is stage 3 onward. Complete Collect as the reference feature before expanding business modules.

## References

- https://www.postgresql.org/support/versioning/
- https://www.postgresql.org/docs/current/ddl-rowsecurity.html
- https://zitadel.com/docs/guides/integrate/login/oidc/oauth-recommended-flows
- https://docs.nats.io/nats-concepts/jetstream/consumers
- https://v2.tauri.app/develop/tests/
