# v2 목표 아키텍처

상태: production target architecture. 실행 가능한 기반은 PR #11에서 CI 검증되었지만 아직 장기 브랜치에 통합되지 않았습니다.

## 원칙

School Collect v2는 MVP 전용 임시 구조를 두지 않습니다. 작은 기능도 최종 trust boundary, tenancy, migration, observability, recovery 방향과 호환되도록 구현합니다.

## 데이터 경로

```text
Tauri (React UI + narrow native adapters)
  -> HTTPS / OIDC access token
  -> Cloudflare edge
  -> Rust/Axum API
       -> SQLx -> PostgreSQL 18
       -> private R2
       -> DB transactional outbox -> relay -> NATS JetStream
                                            -> worker -> R2 / PostgreSQL

System browser -> ZITADEL (OIDC Authorization Code + PKCE)
Local SQLite -> draft/cache/outbox -> API
Server data remains authoritative
```

클라이언트는 DB/R2/NATS 관리자 credential을 갖지 않습니다. Tauri의 Rust 코드도 신뢰 서버가 아닙니다.

## 저장소 구조

현재 확정 방향:

```text
apps/app/                 React/Vite product + Tauri 2 shell
services/api/             Axum HTTP API
services/worker/          asynchronous worker process
services/migrator/        migration-only process

crates/domain/            authoritative business invariants
crates/application/       server use-cases and ports
crates/db/                SQLx repositories/transactions/migrator binding
crates/auth/              auth/authz boundary helpers
crates/contracts/         API/OpenAPI DTO contracts
crates/observability/     tracing/telemetry setup

packages/ui/              approved design-system implementation
packages/schemas/         generated/shared client schemas where justified
packages/config/          non-secret shared build/config primitives

migrations/v2/            v2 PostgreSQL migration source of truth
infra/                    local/staging/production infrastructure definitions
docs/ADR/                 architecture decisions
```

server-only crate/config가 Tauri bundle로 유입되지 않게 합니다.

## Server composition

API는 modular monolith를 기준으로 합니다. worker와 migrator는 책임을 분리한 별도 process입니다.

마이크로서비스 분리는 독립 배포/스케일/장애 경계가 실제로 필요할 때 근거를 갖고 수행합니다.

`/health`는 process liveness, `/ready`는 PostgreSQL 등 required dependency readiness를 나타냅니다. dependency가 준비되지 않았는데 readiness 성공을 반환하지 않습니다.

## PostgreSQL / migration

v2 migration은 `migrations/v2/`만 실행합니다. legacy v1 migration과 섞지 않습니다.

- runtime DB role != migration role
- runtime role은 schema owner/superuser/BYPASSRLS 금지
- 이미 배포된 migration 수정 대신 forward-only migration
- 파괴적 변경은 보존/복구 계획
- tenant-scoped FK/unique/invariant
- 필요 시 FORCE RLS를 defense-in-depth로 사용

production migration에는 demo 업무 데이터를 넣지 않습니다. 테스트 DB 데이터는 runtime factory/builders로 생성 후 정리합니다.

## Authentication / authorization

ZITADEL은 identity provider, School Collect 서버는 tenant membership/RBAC/resource policy authority입니다.

검증 기준:
- Authorization Code + PKCE
- external browser
- state/nonce/redirect 검증
- API에서 signature/issuer/audience/expiration 검증
- verified issuer + subject로 identity 연결
- client가 보낸 tenant_id/role을 신뢰하지 않음

UI 권한 표시는 usability를 위한 것이며 access control이 아닙니다.

## API contract

OpenAPI를 API 계약의 기준으로 사용합니다.

기본 운영 규칙:
- explicit versioning
- structured error code/request id
- bounded pagination
- optimistic version where needed
- idempotency key for retryable mutations
- server-side deadline/state transitions
- sensitive payload redaction

## Async reliability

업무 데이터 변경과 outbox row를 같은 PostgreSQL transaction에서 commit합니다.

```text
business transaction
  -> outbox
  -> relay publish
  -> JetStream
  -> durable consumer
  -> idempotent side effect
  -> ACK after effect commit
```

재전달은 정상 failure mode로 취급합니다. NATS가 exactly-once business side effect를 자동 보장한다고 가정하지 않습니다.

## Files

R2는 private bucket을 기본으로 합니다. 서버가 tenant/resource 권한을 검증한 뒤 제한된 upload/download capability를 제공합니다.

파일 metadata와 access policy는 DB에, object bytes는 R2에 둡니다.

## Local/offline

SQLite는 전체 server database replica가 아닙니다.

허용 책임:
- drafts
- local cache
- pending mutation/outbox
- version/idempotency metadata

사용자/tenant 경계, conflict handling, logout cleanup, retention을 설계합니다. token은 평문 SQLite/localStorage에 두지 않습니다.

## Observability / operations

- structured tracing
- metrics
- secret/PII log redaction
- request/job correlation
- staging/production separation
- backup + restore drill
- signed release/update
- least-privilege network exposure

백업 성공은 “백업 파일이 있다”가 아니라 실제 restore 검증으로 판단합니다.

## Design boundary

Figma -> approved design contract -> repository tokens -> `packages/ui` -> product screens 순으로 관리합니다.

UI primitive에는 business API 호출을 넣지 않습니다.

## Versioning

실제 Rust/Node/pnpm/crate/npm/PostgreSQL image는 lockfile/toolchain/CI로 고정합니다. 문서에 적힌 버전 문자열만으로 reproducibility를 주장하지 않습니다.
