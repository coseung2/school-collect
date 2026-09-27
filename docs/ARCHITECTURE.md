# v2 목표 아키텍처

상태: production target architecture. 실행 기반(PR #11), Code DS/AppShell(PR #17), 인증·수합 기본 흐름(PR #19)은 develop에 통합됐습니다. 현재 구현과 미검증 범위는 [STATUS.md](STATUS.md)를 따릅니다.

## 원칙

School Collect v2는 MVP 전용 임시 구조를 두지 않습니다. 작은 기능도 최종 trust boundary, tenancy, migration, observability, recovery 방향과 호환되도록 구현합니다.

## 학교업무와 개인기능 경계

- 학교업무: 학교 현황·수합·내 제출·구성원은 서버의 계정/membership/권한을 기준으로 합니다. offline 제출 초안도 사용자/학교 범위의 데이터입니다.
- 개인기능: 개인 바로가기와 브라우저 레시피는 School Collect 로그인·학교 가입 없이 사용하는 로컬 도구입니다. 현재 기능 브랜치의 TSV 레시피는 OS 사용자 단위이며 학교 DB에 업로드하지 않습니다.
- AppShell·Design System·기기 설정은 공유하되 학교 로그인/로그아웃·API 장애가 개인 화면을 막지 않는 것을 목표로 합니다. 이 경계는 일부 기능 브랜치에 구현됐으며 develop 통합 전입니다.
- 개인 자동화에서 학교 데이터를 사용하려면 별도 API 권한·보존·정리 계약을 둡니다. 외부 업무 사이트의 브라우저 로그인 세션을 앱 서버로 복사하지 않습니다.

작업 계획: [학교업무](SCHOOL_WORK_PLAN.md), [개인기능](PERSONAL_PLAN.md), [공통 기반](TEAM_BACKLOG.md).

## 학교업무 데이터 경로 (목표)

```text
Tauri (React UI + narrow native adapters)
  -> HTTPS / OIDC access token
  -> Cloudflare edge
  -> Rust/Axum API
       -> SQLx -> PostgreSQL 18
       -> private R2
       -> DB transactional outbox -> relay -> NATS JetStream
                                            -> worker -> R2 / PostgreSQL

System browser -> Supabase Auth (external browser PKCE: planned)
Local SQLite -> draft/cache/outbox -> API
Server data remains authoritative
```

클라이언트는 DB/R2/NATS 관리자 credential을 갖지 않습니다. Tauri의 Rust 코드도 신뢰 서버가 아닙니다.

위 Cloudflare edge/R2/JetStream과 외부 브라우저 PKCE는 목표이며 연결 완료를 의미하지 않습니다. 현재 인증 방식은 ADR-0002의 Supabase 로그인과 API token 검증입니다. 개인기능의 경로는 Tauri -> 로컬 레시피/제한된 브리지(`127.0.0.1:43110`) -> 브라우저 확장(`apps/extension`) -> 현재 탭이며, 브리지·확장은 브랜치에 구현됐고 DOM 자동입력·감시는 후속 계획입니다.

## 저장소 구조

현재 확정 방향:

```text
apps/app/                 React/Vite product + Tauri 2 shell
apps/extension/           Chrome/Edge MV3 extension for the local automation bridge
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

로컬/CI는 PostgreSQL 18입니다. ADR-0002의 외부 DB 실행 기록은 PostgreSQL 17 대상으로, 대상별 버전과 검증 근거를 구분합니다.

- runtime DB role != migration role
- runtime role은 schema owner/superuser/BYPASSRLS 금지
- 이미 배포된 migration 수정 대신 forward-only migration
- 파괴적 변경은 보존/복구 계획
- tenant-scoped FK/unique/invariant
- 필요 시 FORCE RLS를 defense-in-depth로 사용

production migration에는 demo 업무 데이터를 넣지 않습니다. 테스트 DB 데이터는 runtime factory/builders로 생성 후 정리합니다.

## Authentication / authorization

[ADR-0002](ADR/0002-supabase-identity-and-database.md)에 따라 Supabase Auth가 identity provider이고 School Collect 서버가 tenant membership/RBAC/resource policy authority입니다. ZITADEL 선택은 대체됐습니다. 현재 로그인은 Supabase 인증 후 access token을 창 메모리에 보관하는 방식이며 외부 브라우저 PKCE·OS 보안 저장소는 후속입니다.

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

구현된 metadata 계약 (2026-09-27, S-04 1차):

- `school_collect.collect_attachments`가 (tenant, collect, 담당자, item key) 단위로 metadata를 보관합니다: 파일 이름·형식·선언 크기·checksum·object key·상태(`pending`/`stored`/`deleted`)·만료 시각.
- object key는 `tenants/{tenant}/attachments/{attachment_id}`로 서버가 만듭니다. 파일 이름은 표시용이며 경로나 object key가 될 수 없습니다.
- 정책(domain): 파일 1개 최대 10 MiB, 항목당 5개, 제출당 20개, 허용 형식 목록(HWP/HWPX·Office·PDF·PNG/JPEG·text·zip), 보존 180일. 만료된 행은 retention sweep이 bytes를 지운 뒤 row를 지웁니다.
- 접근 규칙: 읽기는 소유자 또는 관리자, 삭제는 관리자 또는 제출 전 소유자. 모든 조회·변경은 tenant로 한정됩니다.
- 아직(2차 이후): storage port와 R2/presigned capability, upload/download API, 화면, worker의 주기 sweep.

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
