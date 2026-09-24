# 팀 기능 분할과 남은 작업

이 문서는 `feat/platform-foundation` 이후 팀이 병렬로 작업할 수 있는 순서와 경계를 고정합니다. 모든 기능 브랜치는 `origin/develop`의 최신 통합 SHA에서 시작하고, 선행 계약을 먼저 병합합니다.

## 먼저 병합할 기반 PR

### 1. Auth request context

권장 브랜치: `feat/stage6-auth-request-context`

소유 경로: `crates/auth`, `services/api`, `crates/contracts`

현재 기반: HTTPS discovery, JWKS, RS256, issuer/audience/exp/nbf 검증, development-only disabled mode, `/v1/auth/principal` probe.

남은 작업:

- OIDC provider를 이용한 실제 signed-token integration test
- JWKS TTL과 rotation 관측성
- issuer/subject를 `users` row로 provision하는 정책
- request ID 생성과 모든 error response의 공통 middleware

완료 기준: 잘못된 서명·issuer·audience·만료 token은 401, provider 장애는 fail closed, token 값은 로그에 남지 않음.

### 2. Tenant membership and RBAC

권장 브랜치: `feat/stage6-tenant-membership`

소유 경로: `crates/application`, `crates/db`, `crates/auth`, `migrations/v2`

선행: Auth request context

남은 작업:

- issuer/subject → user upsert
- membership 조회와 role policy
- tenant context를 모든 repository query에 전달
- `admin`, `coordinator`, `contributor`, `viewer` negative test
- pooled connection에서 tenant scope가 누출되지 않는 integration test
- 필요 시 transaction-scoped PostgreSQL RLS defense-in-depth

완료 기준: tenant A/B 격리와 role 상승 차단을 실제 PostgreSQL 테스트로 증명.

## 병렬 구현 가능한 기능 PR

### 3. Collect admin flow

권장 브랜치: `feat/collect-admin-flow`

소유 경로: `crates/domain`, `crates/application`, `crates/db`, `services/api`, `crates/contracts`

선행: Tenant membership and RBAC

범위: draft 생성, item 정의, publish, assignment 생성, close.

완료 기준: `draft -> published -> closed`만 허용하고, 다른 tenant와 contributor가 관리자 mutation을 수행하지 못함.

### 4. Collect contributor flow

권장 브랜치: `feat/collect-submission-flow`

소유 경로: `crates/application`, `crates/db`, `services/api`, `apps/app`

선행: Collect admin flow

범위: draft 저장, 재개, version conflict, submit, closed 이후 차단.

완료 기준: 중복 요청과 오래된 version이 안전하게 거부되고, 재시작 후 draft를 다시 읽을 수 있음.

### 5. Outbox and worker

권장 브랜치: `feat/outbox-worker-foundation`

소유 경로: `crates/db`, `services/worker`, `infra`, `docs/ADR`

선행: Collect admin flow

범위: business mutation과 outbox INSERT의 단일 transaction, relay, JetStream durable consumer, idempotent handler, retry/dead-letter.

완료 기준: worker가 같은 event를 여러 번 받아도 side effect가 한 번만 적용되고 ACK는 commit 뒤에만 발생함.

### 6. Desktop auth and offline draft

권장 브랜치: `feat/app-auth-offline-draft`

소유 경로: `apps/app`, `apps/app/src-tauri`, `packages/ui`

선행: Auth request context, Collect contributor flow

범위: external browser PKCE callback, OS secure storage adapter, SQLite draft/outbox, logout cleanup, offline/online conflict UI.

완료 기준: token이 localStorage/평문 SQLite에 저장되지 않고, 네트워크 단절 후 재연결 시 version conflict를 사용자에게 표시함.

## 운영과 배포에서 남은 작업

- private R2 bucket, signed upload/download, retention 정책
- staging/production API와 CORS/CSP 분리
- runtime DB role과 migration role 분리
- structured log redaction, metrics, tracing, request/job correlation
- PostgreSQL backup/restore drill과 RPO/RTO 기록
- Tauri signing/update verification과 mobile target 검증
- 외부 credential 폐기·회전과 Git history rewrite는 별도 승인된 보안 작업

운영 계정이나 유료 리소스를 저장소 작업의 부수 효과로 생성하지 않습니다.
