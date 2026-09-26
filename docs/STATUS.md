# 실행 상태

기준일: 2026-09-27 (Asia/Seoul). 이 문서는 계획, 기능 브랜치 구현, develop 통합과 실제 검증을 구분합니다.

작업 분류는 [V2_PLAN.md](V2_PLAN.md), 상세 계획은 [학교업무](SCHOOL_WORK_PLAN.md) / [개인기능](PERSONAL_PLAN.md), 공통 기반은 [TEAM_BACKLOG.md](TEAM_BACKLOG.md)를 따릅니다. `내 제출`과 offline 제출 초안은 학교업무이며 개인 바로가기·자동입력·감시는 개인기능입니다.

## 이번에 확인한 범위

- `git fetch origin` 후 `origin/develop` `26aa851`, `origin/main` `49f856a`를 확인했습니다.
- GitHub 조회에서 PR #24/#25가 모두 `develop` 대상 OPEN이고 각 head의 CI 6개가 성공한 것을 확인했습니다.
- 해당 브랜치 문서·route/native 실행 경계와 develop의 사용자/membership·API request ID·worker 코드를 대조했습니다.
- 아래 과거 native/API/DB·브라우저 검증 및 외부 운영 설정 기록은 이번에 재실행·재조회하지 않았습니다. CI 성공도 production 배포나 실제 창 조작 검증을 대신하지 않습니다.

## 학교업무 / 개인기능 상태

| 영역 | 통합된 것 | 통합 전 구현 | 남은 우선 작업 |
| --- | --- | --- | --- |
| 학교업무 | PR #19 인증·학교 등록·수합 생성/배포/초안/제출/마감 | PR #24 `1dd656f`: 항목·대상·제출 현황·내 배정·구성원 화면 | S-01 통합 검증 -> S-02 교사 초대·합류 |
| 개인기능 | develop에 개인 바로가기 미통합 | PR #25 `fc944b9`: 로컬 바로가기; 로컬 `c31b9d9`: 로그인 없는 개인 화면·recipe ID 실행 | P-01 안정화·native 검증 -> P-02 현재 화면 등록 |
| 공통 기반 | PR #17 DS/AppShell, PR #20 개발자별 Docker 구성 | PR #24와 로컬 `c31b9d9`의 화면 접근 경계 | C-01 영역·설정·offline 구분, C-02~C-05 보안·운영·배포 |

PR #24/#25는 열려 있으며 로컬 `feat/personal-automation`은 학교 화면 변경도 포함합니다. 이 구현들을 develop 완료 상태로 합산하지 않습니다.

## 단계 상태

| 단계 | 상태 | 확인된 내용 | 남은 게이트 |
| --- | --- | --- | --- |
| 1. 보안/기준점 | 추적 파일 정리 완료 | public 저장소 확인, PR #10 main merge, PR #12 동기화, v1 애플리케이션·업무자료 추적 제거, Infisical dev 서비스 토큰 회전(2026-09-25) | history rewrite 범위/실행, 나머지 외부 credential 정리 |
| 2. 협업 | 보호 적용 완료 | develop/작업 브랜치/문서/CI 구성, main·develop branch protection 적용 및 API 확인 | 협업자 추가 시 approval >= 1 및 code owner review, 독립 리뷰 |
| 3. 실행 기반 | 조건 충족 | lockfile 커밋 + frozen/locked CI, TS strict, fmt/clippy/test, PostgreSQL 18 migration, /health·/ready 분리 검증, Windows native build, local tauri dev | mobile/Linux/macOS 미검증, client production CSP·환경 분리 후속 |
| 4. Figma | 시안 승인 | Design System/Product 핵심 UI 승인, Stage 5 기본 token/component mapping | interaction/accessibility detail, Library/Code Connect 후속 |
| 5. Code DS/AppShell | 완료 (PR #17, `9697363`) | `packages/ui` token/component와 실제 Tauri AppShell, 상태·키보드·wide/narrow 검증 | Figma Library/Code Connect 후속 |
| 6. Auth/Data/Ops | 구현 중 | Supabase Auth 실로그인 + ES256/JWKS 검증, user/membership provisioning, tenant RBAC, Collect 상태 전이와 version 충돌까지 실제 프로젝트·실제 DB에서 E2E 통과 | R2/NATS/SQLite/backup, 환경 분리, 세션 영속화 |
| 7. Collect | 구현 중 | PR #19 기본 흐름 통합, 항목·대상·현황 확장은 PR #24 리뷰 대기 | 구성원 초대, 항목/대상 편집, 첨부, offline, 결과 export 및 전체 통합 E2E |

## 확인된 원격 상태

### Public repository / security
- repository visibility: public
- public 유지가 사용자 의도임
- 과거 `.env.example` 이력에서 실제처럼 보이는 외부 서비스/DB credential 형태 값 확인
- 과거 local CLI state에 project connection metadata 추적 이력 확인
- `_agent_작업` 아래 실제 업무에서 생성된 source/derived artifact 존재 확인
- 현재 HEAD 노출 축소용 hotfix PR #10이 main에 merge됨 (merge commit `3a7acd9`)
- PR #12로 main -> develop 동기화 완료 (merge commit)
- 이후 소유자 결정으로 v1 웹 애플리케이션과 `_agent_작업` 업무자료를 추적 대상에서 전부 제거. 현재 HEAD에는 실제 업무자료가 없음
- 추적 제거는 과거 Git object를 삭제하지 않으므로 history 정리는 여전히 별도 작업

credential 값 자체는 문서에 기록하지 않습니다.

### 승격 이력

- `develop` 통합: PR #19(실제 Supabase 인증·DB 연결과 첫 Collect 수직 슬라이스), PR #20(개발자별 Docker 개발 스택과 로컬 실행 절차)을 squash merge했습니다.
- `develop -> main` 승격: PR #21. 통합 후보 SHA는 `3ac8d1f`이고 merge commit은 `49f856a`입니다. 승격 후 main CI(Foundation) success를 확인했습니다.
- `main -> develop` 동기화: PR #22. merge commit은 `b50ff87`이고, 승격 merge commit이 develop에 포함됩니다. 두 브랜치의 트리는 동일합니다.
- 내용 변경 없는 승격·동기화이므로 이 시점의 배포나 production migration은 없습니다.

### Git protection
classic branch protection을 `main`과 `develop`에 적용하고 API 조회로 확인했습니다. pull request 필수, stale approval dismissal, conversation resolution, required check `repository-checks`, force push/삭제 차단, enforce_admins가 두 브랜치 모두 활성입니다. `main`만 strict(최신 base 강제)를 적용했습니다.

approval 최소 개수는 0입니다. 단독 운영 상태에서 1 이상이면 본인 PR을 병합할 수 없기 때문이며, 협업자 추가 시 함께 올려야 합니다. 자세한 내용은 [BRANCH_PROTECTION.md](BRANCH_PROTECTION.md)를 봅니다.

### Vercel 연동
v2 전환 후 남아 있던 Vercel `school-collect` 프로젝트와 배포를 2026-09-23에 삭제했습니다. 이 저장소는 Vercel 배포를 사용하지 않습니다.

### Stage 3
PR #11이 develop에 squash merge되었습니다 (`4cadf7e`).

Stage 3 완료 항목:
- `pnpm-lock.yaml`(root + `apps/app` importer)과 `Cargo.lock`을 커밋해 source of truth로 확정
- CI의 모든 install을 `pnpm install --frozen-lockfile` / cargo `--locked`로 전환
- `--exclude school-collect-app` 제거. clippy `-D warnings`와 test가 Tauri crate 포함
- 실제 개발 머신에서 `tauri dev` 확인. "School Collect" 창 표시, Vite dev server 200 응답
- 실제 개발 머신에서 `tauri build --no-bundle` 성공
- PostgreSQL 18 빈 DB 확인 → v2 migration → 결과 검증 → 재실행 idempotency를 CI에서 검증
- production migration에 demo seed table이 생성되지 않음을 CI 검사로 보장
- `/health`와 `/ready` 의미 분리를 실제로 검증. `DATABASE_URL` 누락은 기동 실패, 도달 불가 DB는 `/ready` 503, 도달 가능 시 200

Stage 3 미검증 항목:
- mobile target/plugin compatibility: Android SDK/NDK 부재, Rust target 미설치, iOS는 macOS 필요. CLI에 `android init`/`ios init`가 존재하는 것만 확인
- dev/prod config 분리: `tauri.conf.json` CSP `connect-src`가 아직 `http://127.0.0.1:*` 허용. production 교체는 Stage 6
- Linux/macOS native build 미검증

### Stage 4 Figma
사용자가 현재 Figma 시안을 승인함.

승인 핵심:
- light compact sidebar
- vertical active indicator 제거
- selective Card
- content/action density 기반 card sizing
- peer collection의 단독 Card 강조 금지
- List Surface + uniform List Row
- semantic state로 priority/urgency 표현
- radius <= 8px
- decorative shadow 없음

## Stage 5 Code Design System / AppShell

첫 번째 code-only migration은 PR #17에서 완료되었습니다.

- `packages/ui`: semantic token, Button, Status, Card, List Surface/Row, Tabs,
  DataTable, FormField, Sidebar, Header, AppShell, Empty/Loading/Error/
  Permission/Offline state primitive
- `apps/app`: 승인된 compact shell, hash 기반 navigation/history, 실제 `/health`
  확인, API error와 browser offline recovery state
- 검증: typecheck, frozen install, build, browser 1280/1024/720 viewport,
  keyboard focus, offline event
- 이 배치는 Figma canonical file을 쓰지 않으며, 실제 업무 데이터나 demo seed를
  추가하지 않습니다. 상세 mapping과 rollback은
  [STAGE5_CODE_MIGRATION.md](STAGE5_CODE_MIGRATION.md)에 기록합니다.

기본 Stage 5는 PR #17로 통합됐습니다. 이후 C-01에서 새 학교업무/개인기능의 진입 경계와 Figma interaction/accessibility·Library/Code Connect를 보강합니다. 과거 완료된 기본 구현을 다시 미착수로 표시하지 않습니다.

## Production-first 데이터 정책

- production migration에 demo school/user/task/submission seed를 넣지 않음
- 저장소에 가상 fixture DB나 실제처럼 채운 demo dataset을 상시 보존하지 않음
- 테스트 실행 시 필요한 데이터만 생성하고 rollback/truncate/disposable DB 등으로 제거
- 실제 업무자료/개인정보는 public repository에 두지 않음
- 제품에 필수인 reference data는 demo data와 구분

## 실제 연결된 개발 환경

- 개발 환경: 개발자별 Docker 스택(`infra/compose.dev.yml`)입니다. 공용 개발 서버와 공용 개발 DB를 두지 않습니다.
- identity/database: 등록된 Supabase 프로젝트(Andong ICT Infisical `school collect`, Development)
- v2 객체는 `school_collect` 스키마에 있고, v1 잔여 테이블(`public`)과 분리되어 있습니다.
- 검증: `services/api/tests/collect_flow_e2e.rs`가 실제 로그인 → 토큰 검증 → 수합 생성·배포·작성·제출·마감과 tenant 격리를 확인하고 생성한 행과 테스트 계정을 모두 삭제합니다.
- 위험: dev/staging/prod Infisical 라벨이 **같은** Supabase 프로젝트를 가리킵니다. 환경 분리는 아직 없습니다.
- 참고: staging용 Supabase 프로젝트 생성은 무료 한도(계정당 무료 프로젝트 2개)에 걸려 있어 소유자 결정이 필요합니다.

## 데스크톱 클라이언트 (Stage 5 이후 첫 제품 기능)

- `apps/app`은 로그인 → 학교 등록 → 수합 생성 → 배포 → 초안 저장 → 제출을 실제 API와 연결합니다.
- 접근 토큰은 창 메모리에만 보관하고 디스크에 저장하지 않습니다.
- 검증: 로컬 API(`APP_AUTH_MODE=oidc`)와 dev 서버를 띄운 상태에서 실제 브라우저로 전체 흐름을 확인했습니다. 검증용 계정과 데이터는 삭제했습니다.

## 통합 전 기능의 검증 기록

### 학교업무 확장: S-01

- PR #24(`feat/collect-mvp-frame`, `1dd656f`)는 항목·대상 저장, 제출 현황, 내 배정, 구성원 화면을 추가합니다.
- 브랜치 문서에는 실제 Supabase/PostgreSQL E2E와 로컬 UI에서 수합 생성 -> 배포 -> 초안 -> 제출 -> 현황 확인을 수행했다고 기록돼 있습니다. 이번에는 이 실행을 재검증하지 않았습니다.
- 교사 초대 경로·첨부·export·offline 초안은 아직 없습니다. develop 통합과 통합 SHA 검증 뒤 S-02를 진행합니다.

### 개인 바로가기: P-01

- PR #25(`feat/work-automation-shortcuts`, `fc944b9`)는 이름/URL 기반 로컬 바로가기와 파일 복구 구현입니다.
- 로컬 `feat/personal-automation` `c31b9d9`에는 로그인 없는 `#automation` 화면과 `open_automation_recipe(recipeId)` 경계가 있습니다. native가 저장된 URL을 다시 읽어 실행합니다.
- 로컬 브랜치 문서에는 native 단위 테스트 15개, fmt/clippy, typecheck/build, foundation 21개 통과 기록이 있습니다. 이번 재실행 결과는 아닙니다. 실제 Tauri 창 등록·열기 smoke는 해당 기록에서도 미실행입니다.
- 현재 TSV 레시피는 OS 사용자 단위이며 학교 계정과 무관합니다. 브라우저 확장·DOM 입력·행렬 입력·감시는 미구현입니다.
- URL 정규화·opener 실행·파일 권한/용량 제한 등 남은 보강은 [PERSONAL_PLAN.md](PERSONAL_PLAN.md)의 P-01을 따릅니다.

## 배포 기반

- `infra/api.Dockerfile`과 `infra/compose.deploy.yml`로 서버 이미지·구성을 정의했습니다.
- 실행 순서와 소유자 결정이 필요한 항목은 [DEPLOYMENT.md](DEPLOYMENT.md)에 정리했습니다.
- 호스팅 위치, 도메인·TLS, DB 역할 분리, 데스크톱 서명은 아직 결정되지 않았습니다.

## 아직 하지 않은 것

- Git history rewrite
- 나머지 외부 credential 실제 폐기/회전 확인 (Infisical dev 서비스 토큰은 2026-09-25 회전 기록 있음)
- production infra 생성/배포
- R2/NATS 업무 처리 및 production 연결, 학교 환경 분리
- production DB migration

인증 provider는 [ADR-0002](ADR/0002-supabase-identity-and-database.md)의 Supabase Auth이며 ZITADEL 구축은 현행 대기 작업이 아닙니다. 현재 클라이언트 토큰은 메모리에만 있고 PKCE·OS 보안 저장소·세션 영속화는 S-05 후속입니다.
