# 실행 상태

기준일: 2026-09-22. 이 문서는 계획과 실제 검증 완료를 구분합니다.

## 단계 상태

| 단계 | 상태 | 확인된 내용 | 남은 게이트 |
| --- | --- | --- | --- |
| 1. 보안/기준점 | 추적 파일 정리 완료 | public 저장소 확인, PR #10 main merge, PR #12 동기화, v1 애플리케이션·업무자료 추적 제거 | credential 폐기/회전, history rewrite 범위/실행 |
| 2. 협업 | 보호 적용 완료 | develop/작업 브랜치/문서/CI 구성, main·develop branch protection 적용 및 API 확인 | 협업자 추가 시 approval >= 1 및 code owner review, 독립 리뷰 |
| 3. 실행 기반 | 조건 충족 | lockfile 커밋 + frozen/locked CI, TS strict, fmt/clippy/test, PostgreSQL 18 migration, /health·/ready 분리 검증, Windows native build, local tauri dev | mobile target/plugin (미검증), dev/prod config 분리 완료 (Stage 6) |
| 4. Figma | 시안 승인 | Design System/Product 파일과 핵심 UI 규칙 승인 | code mapping, interaction/accessibility detail, Library/Code Connect 후속 |
| 5. Code DS/AppShell | 구현 중 | `packages/ui` token/component와 실제 AppShell 1차 배치, wide/narrow/offline/browser/native dev 검증 | CI/PR 검증, Figma와의 상세 interaction/accessibility 대조 |
| 6. Auth/Data/Ops | 계획 | architecture 결정 | OIDC/RBAC/R2/NATS/SQLite/backup 실제 구현·검증 |
| 7. Collect | 계획 | reference flow 정의 | end-to-end production-grade 구현/E2E |

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

### Git protection
classic branch protection을 `main`과 `develop`에 적용하고 API 조회로 확인했습니다. pull request 필수, stale approval dismissal, conversation resolution, required check `repository-checks`, force push/삭제 차단, enforce_admins가 두 브랜치 모두 활성입니다. `main`만 strict(최신 base 강제)를 적용했습니다.

approval 최소 개수는 0입니다. 단독 운영 상태에서 1 이상이면 본인 PR을 병합할 수 없기 때문이며, 협업자 추가 시 함께 올려야 합니다. 자세한 내용은 [BRANCH_PROTECTION.md](BRANCH_PROTECTION.md)를 봅니다.

### v1 Vercel 연동
v1 웹 애플리케이션 제거와 함께 Vercel Preview 빌드를 깨뜨리던 원인(`supabaseUrl is required.`)도 저장소에서 사라졌습니다. Vercel 프로젝트 연결과 Production 환경변수는 저장소 밖 설정이므로, 연결 해제 또는 프로젝트 정리는 소유자가 Vercel 쪽에서 별도로 수행해야 합니다.

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

첫 번째 code-only migration은 별도 작업 브랜치에서 진행 중입니다.

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

남은 Stage 5 게이트는 CI 결과와 Figma 승인 화면 및 세부
interaction/accessibility의 대조입니다.

## Production-first 데이터 정책

- production migration에 demo school/user/task/submission seed를 넣지 않음
- 저장소에 가상 fixture DB나 실제처럼 채운 demo dataset을 상시 보존하지 않음
- 테스트 실행 시 필요한 데이터만 생성하고 rollback/truncate/disposable DB 등으로 제거
- 실제 업무자료/개인정보는 public repository에 두지 않음
- 제품에 필수인 reference data는 demo data와 구분

## 아직 하지 않은 것

- Git history rewrite
- 외부 credential 실제 폐기/회전
- Vercel 프로젝트 연결 해제/정리 (저장소 밖 설정)
- production infra 생성/배포
- ZITADEL/R2/NATS production 연결
- production DB migration
