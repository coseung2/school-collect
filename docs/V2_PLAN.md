# School Collect v2: 7단계 실행 계획

기준일: 2026-09-22. 상위 이슈: #1.

v2는 MVP나 임시 프로토타입을 목표로 하지 않습니다. 각 단계에서 구현하는 범위는 작게 유지할 수 있지만, 구조는 production 기준의 최종 방향과 호환되어야 합니다.

## 상태 표현

`계획` / `구현 중` / `검증 중` / `리뷰 대기` / `완료`를 구분합니다.

설정 파일, scaffold, mock, 문서만으로 실행 완료를 주장하지 않습니다. 단계별로 PR, 검증 commit SHA, 실제 실행 명령/결과, 리뷰 기록, 남은 위험을 남깁니다. 현재 사실 상태는 [STATUS.md](STATUS.md)를 기준으로 합니다.

## 단계와 게이트

| 단계 | 이슈 | 결과물 | 완료 게이트 |
| --- | --- | --- | --- |
| 1. 기준점·보안 | #2 | public OSS에 적합한 저장소/history 정리 | credential 폐기/회전, 실자료 history 처리, 보존 결정 |
| 2. 공동작업 | #3 | develop, 규칙, templates, CODEOWNERS, CI | 실제 branch protection/ruleset 및 리뷰 차단 확인 |
| 3. 실행 기반 | #4 | Tauri/Rust/PostgreSQL workspace | reproducible install/build/test, migration/native smoke |
| 4. Figma | #5 | Design System + Product 기준안 | 실제 Figma 파일 검수와 사용자 승인 |
| 5. Code DS / AppShell | #6 | code tokens/UI/AppShell | native smoke, accessibility/state 검수 |
| 6. Auth/Data/Operations | #7 | authz/data/jobs/offline/backup 기반 | 보안 부정 테스트, 장애/복원 검증 |
| 7. Collect reference implementation | #8 | end-to-end 기준 구현 | 통합 SHA + native/API/DB/E2E 검증 |

사용자 결정에 따라 Figma 기준안을 실행 기반과 병행하여 먼저 확정했으며, Stage 4 시안은 승인되었습니다. Stage 3 실행 기반은 PR #11에서 CI 검증 중/완료 항목을 축적하고 있습니다.

## 1. 기준점·보안

저장소는 public 오픈소스 상태를 유지합니다. 목표는 private 전환이 아니라 public에 적합하지 않은 legacy credential과 실제 업무 source/derived artifact를 안전하게 제거하는 것입니다.

- 신규 secret/실자료 유입 차단
- 현재 HEAD sanitization
- 과거 Git history 대상 분류
- 외부 credential 폐기/회전
- history rewrite 필요 범위 확정
- rewrite 시 백업/협업자 재동기화/공개 캐시 한계 기록

Supabase는 폐기된 legacy 인프라입니다. 복구/재연결하지 않습니다.

production용 seed에 demo data를 넣지 않습니다. 테스트 데이터는 테스트 실행 중 생성하고 종료 시 제거합니다.

## 2. 공동작업

`feat/fix/refactor/chore/docs -> develop -> main`을 기본으로 합니다.

- 작업 브랜치 -> develop: squash
- develop -> main: merge commit
- main -> develop: release/hotfix 이후 merge commit sync
- hotfix: main에서 분기

repository ruleset/branch protection은 문서가 아니라 GitHub 서버 설정으로 검증합니다. 현재 사실 상태는 STATUS/BRANCH_PROTECTION 문서를 따릅니다.

## 3. 실행 기반

목표는 modular monolith API + 별도 worker + migration-only process입니다.

현재 실행 기반 방향:
- `apps/app`: React/Vite + Tauri 2
- `services/api`: Axum
- `services/worker`: async worker seam
- `services/migrator`: migration-only
- `crates/domain|application|db|auth|contracts|observability`
- PostgreSQL 18
- v2 migration: `migrations/v2/`
- liveness `/health`, dependency readiness `/ready`
- generated OpenAPI

완료 게이트:
- pnpm/Cargo lockfile 고정과 frozen/locked CI
- TS build/typecheck
- Rust fmt/clippy/test
- 빈 PostgreSQL 18에 v2 migration
- API readiness
- Windows native Tauri smoke
- 실제 개발 환경 `tauri dev` smoke
- mobile target/plugin compatibility 확인

테스트용 DB 레코드는 실행 중 생성·정리하며 저장소에 상시 seed하지 않습니다.

## 4. Figma

파일:
- `School Collect — Design System`
- `School Collect — Product`

시안은 사용자 승인 완료 상태입니다.

핵심 승인 규칙:
- light compact sidebar, vertical active indicator 없음
- one primary action hierarchy
- radius <= 8px
- decorative shadow 없음
- Card는 독립 정보/행동 객체
- 카드 크기는 정보/행동 밀도에 비례
- peer collection 안에서 한 항목만 Card로 승격하지 않음
- shared List Surface + uniform List Row
- urgency는 semantic state로 표현
- Default/Loading/Empty/Error/Permission/Offline 상태 포함

남은 항목은 code mapping, interaction/accessibility detail, Library/Code Connect 등이며 Stage 5와 연결합니다.

## 5. Code Design System / AppShell

승인 Figma를 저장소 token과 `packages/ui`로 옮깁니다.

기준 컴포넌트:
Button, FormField, Status, Card, ListSurface, ListRow, Sidebar, Header, Tabs, Table, Dialog, Sheet, Toast, Empty/Loading/Error/Permission/Offline.

화면별 임의 값보다 token/component를 우선합니다. 제품 화면에서 새 reusable rule이 생기면 Design System에 함께 반영합니다.

AppShell은 실제 Tauri 창, routing, navigation, error/offline state와 native capability 최소화를 포함합니다.

## 6. Auth / Data / Operations

### 6A Authentication
ZITADEL OIDC, external browser, Authorization Code + PKCE, state/nonce/redirect 검증, API token validation, secure token storage.

### 6B Authorization / PostgreSQL
organization/school membership, RBAC, tenant-scoped invariant, runtime/migrator role 분리, audit, 필요 시 RLS defense-in-depth.

### 6C Files
private R2, 서버 권한 확인, 제한된 upload/download, lifecycle/retention.

### 6D Async
업무 DB 변경과 outbox INSERT를 한 transaction으로 commit. relay -> NATS JetStream -> idempotent worker. ACK는 side effect commit 후 수행합니다.

### 6E Local/offline
SQLite는 draft/cache/outbox 역할만 가집니다. 서버가 최종 데이터 기준입니다. idempotency/version/conflict/logout cleanup을 설계합니다.

### 6F Operations
staging/production 분리, tracing/metrics/log redaction, backup/restore drill, signed release/update.

운영 리소스는 대상 계정과 권한이 확정되기 전 임의로 생성하지 않습니다.

## 7. Collect reference implementation

Collect는 “MVP”가 아니라 이후 기능이 따라야 할 **production-grade reference implementation**입니다.

관리자 생성 -> 배포 -> 교사 draft -> 제출 -> 담당자 status -> 마감 -> 결과/export 흐름을 DB/domain/API/client/offline/UI/audit/security/test까지 수직으로 구현합니다.

검증 항목:
- tenant A/B 격리
- 교사/담당자/관리자 권한
- 마감 이후 제출
- 중복 요청/idempotency
- 동시 수정
- 네트워크 단절/재전송
- 재시작 draft 복구
- export 재처리
- 서버 기준 시간/상태 전이

테스트 데이터는 테스트 수명 동안만 존재해야 합니다.

## 공통 원칙

- production-first
- server-authoritative security
- public repository에 secret/실자료 없음
- persistent demo seed 없음
- runtime-generated test data + cleanup
- 실제 실행한 검증만 기록
- 운영 변경은 별도 권한
- 데이터 삭제/history rewrite는 별도 보안 작업
