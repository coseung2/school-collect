# v2 목표 아키텍처

상태: 설계 기준. 아래 서비스와 경로는 아직 구현·배포 완료가 아닙니다.

## 데이터 경로

```text
Tauri (React UI + narrow native adapters)
  -> HTTPS / OIDC access token
  -> Cloudflare edge
  -> Rust/Axum API
       -> SQLx -> PostgreSQL
       -> private R2
       -> DB transactional outbox -> relay -> NATS JetStream
                                            -> job worker -> R2 / PostgreSQL

System browser -> ZITADEL (OIDC Authorization Code + PKCE)
Local SQLite -> draft/outbox -> API; server is authoritative
```

DB credential, R2 signing credential, ZITADEL masterkey, NATS credentials는 서버에만 둡니다. 클라이언트 Rust는 신뢰 서버가 아닙니다. 일반 CRUD도 API 권한 검사를 거칩니다.

## 목표 디렉터리와 책임

```text
apps/app/                 React/Vite product + src-tauri native shell
packages/ui/              reusable UI, no business API dependency
packages/tokens/          approved token manifest / generated CSS
packages/core/            client use-cases, draft/view logic; not authorization authority
packages/data/            generated API client wrapper / platform adapters
packages/schemas/         generated API types and client validation
services/api/             Axum HTTP, auth, tenant context, application composition
services/worker/          isolated async consumers
crates/domain/            authoritative business rules, no HTTP/DB dependency
crates/application/       server use-cases and ports
crates/persistence/       SQLx repositories and transaction boundaries
crates/contracts/         DTO/OpenAPI definitions, no secrets/config
db/migrations/            v2 database source of truth
infra/                    dev, staging/prod manifests and runbooks
docs/ADR/                 versioned architecture decisions
```

서버가 규칙의 최종 판단자입니다. TS 검증은 UX 보조이며 Rust 권한 로직을 대체하지 않습니다. OpenAPI에서 TS 타입/클라이언트를 생성하는 것을 기준으로 하고, 필요한 코드 공유만 합니다. 클라이언트 crate가 서버 설정·DB/인증 관리자 dependency를 끌고 오지 않게 합니다.

## 서버 구성

API는 modular monolith로 시작하고 worker는 별도 실행 프로세스로 분리합니다. Docker/Compose는 로컬 개발과 검증의 수단입니다. API/DB 분리만으로 HA가 보장되지는 않습니다. 장애 도메인, replica, 백업, 복원, 배포 절차를 따로 검증합니다.

Caddy/TLS 종료와 Cloudflare proxy/origin 접근 정책은 실제 배포 시 결정합니다. DB와 NATS 관리 포트를 공용 인터넷에 노출하지 않습니다. Cloudflare를 거친다고 접근권한 검사가 생기는 것은 아닙니다. API, DB, worker는 데이터 지연과 운영 요구를 고려해 가까운 리전에 둡니다. 국내 저장·국외 처리 제한 등은 실제 조직의 정책을 먼저 확인하며 R2 사용이 자동으로 해당 정책을 만족한다고 간주하지 않습니다.

## 인증 및 tenant authorization

ZITADEL은 신원 인증, School Collect는 학교 소속/역할/업무 권한을 관리합니다. 이메일만으로 계정을 식별하지 않고 검증된 issuer+subject를 사용합니다. access token인지 ID token인지 구분하여 검증합니다.

클라이언트의 tenant_id/role은 신뢰하지 않습니다. 서버가 membership을 확인하고, 동일 DB transaction 안에서 transaction-local tenant context를 설정합니다. runtime DB 역할은 table owner/superuser/BYPASSRLS가 아니어야 하며, 필요한 테이블에는 FORCE RLS를 검토합니다. pool 재사용과 A/B tenant 교차 테스트를 필수로 둡니다. tenant-scoped FK/unique constraint로 학교 간 잘못된 관계도 차단합니다.

## API/업무 처리

`/v1` 계약, 입력 검증, 제한된 pagination, 일관된 오류 코드/request id, optimistic version, idempotency key를 사용합니다. deadline과 상태 전이는 서버 기준입니다. 요청 전체/토큰/개인정보 원문을 자동 로깅하지 않습니다. audit에는 actor, action, resource, 시점, 최소 변경 요약을 기록합니다.

## 비동기 신뢰성

DB 변경과 메시지 전송을 별도 성공 조건으로 처리하지 않습니다. 업무 데이터와 outbox를 한 DB transaction으로 commit하고 relay가 JetStream publish ACK 후 전송 상태를 기록합니다. 재전달은 정상적인 실패 시나리오로 간주합니다.

Consumer는 durable/pull 및 explicit ACK를 기준으로 합니다. idempotent effect, job unique key, 재시도/backoff, 최대 횟수, 실패함으로의 이동, 관측성을 구현합니다. NATS 전달 보장만으로 DB/파일 side effect의 exactly-once를 주장하지 않습니다. 문서 변환은 격리된 worker와 제한된 리소스/파일 범위에서 수행합니다.

## 로컬/모바일

SQLite는 초안·작업 outbox·cache 용도입니다. 서버와 전 DB를 복제하는 sync engine은 이번 범위가 아닙니다. 로컬에도 학교/사용자 구분, 보존·삭제·충돌 정책이 필요합니다. refresh token은 일반 localStorage/평문 SQLite에 넣지 않고 검증된 OS 보호 저장소 adapter로 다룹니다.

Desktop/mobile별 plugin 지원과 보안 저장소/파일 picker/redirect 제약은 실제 target에서 검증합니다. 화면 크기만 줄였다고 모바일 대응이 완료된 것은 아닙니다.

## 버전과 출처

PostgreSQL 공식 지원표는 2026-09-21 조회 시 18.6을 표시합니다. 실제 이미지 digest, Rust/Node/pnpm과 crate/npm 버전은 실행 기반 PR에서 lockfile 및 CI로 검증해 고정합니다.

- PostgreSQL 지원: https://www.postgresql.org/support/versioning/
- PostgreSQL RLS: https://www.postgresql.org/docs/current/ddl-rowsecurity.html
- ZITADEL 권장 흐름: https://zitadel.com/docs/guides/integrate/login/oidc/oauth-recommended-flows
- ZITADEL self-hosting: https://zitadel.com/docs/self-hosting/deploy/compose
- NATS consumers: https://docs.nats.io/nats-concepts/jetstream/consumers
- Tauri tests: https://v2.tauri.app/develop/tests/
