# 실행 상태

기준일: 2026-09-22. 이 문서는 계획과 실제 검증 완료를 구분합니다.

## 단계 상태

| 단계 | 상태 | 확인된 내용 | 남은 게이트 |
| --- | --- | --- | --- |
| 1. 보안/기준점 | 현재 HEAD 정리 완료 | public 저장소 확인, PR #10 main merge, PR #12로 main -> develop 동기화 | credential 폐기/회전, 업무자료 history 처리, history rewrite 범위/실행 |
| 2. 협업 | 진행 중 | develop/작업 브랜치/문서/초기 CI 구성 | main/develop 실제 보호 설정, 독립 리뷰 |
| 3. 실행 기반 | 검증 중 | PR #11에서 TS/Rust/Postgres/Windows Tauri CI 성공 | lockfile final pin, frozen/locked CI, local tauri dev, mobile compatibility |
| 4. Figma | 시안 승인 | Design System/Product 파일과 핵심 UI 규칙 승인 | code mapping, interaction/accessibility detail, Library/Code Connect 후속 |
| 5. Code DS/AppShell | 미착수 | 승인 Figma 기준 존재 | packages/ui, token mapping, 실제 AppShell |
| 6. Auth/Data/Ops | 계획 | architecture 결정 | OIDC/RBAC/R2/NATS/SQLite/backup 실제 구현·검증 |
| 7. Collect | 계획 | reference flow 정의 | end-to-end production-grade 구현/E2E |

## 확인된 원격 상태

### Public repository / security
- repository visibility: public
- public 유지가 사용자 의도임
- legacy `.env.example` 과거 이력에서 실제처럼 보이는 외부 서비스/DB credential 형태 값 확인
- legacy local CLI state에 project connection metadata 추적 이력 확인
- `_agent_작업` 아래 실제 업무에서 생성된 source/derived artifact 존재 확인
- 현재 HEAD 노출 축소용 hotfix PR #10이 main에 merge됨 (merge commit `3a7acd9`)
- PR #12로 main -> develop 동기화 완료 (merge commit)
- PR #10은 과거 Git object를 제거하지 않으므로 history 정리는 별도

credential 값 자체는 문서에 기록하지 않습니다.

### Git protection
- repository rulesets 조회 결과: 비어 있음
- classic branch protection API는 연결 권한상 403으로 확인/적용 불가
- 따라서 main/develop 보호가 활성화되었다고 주장하지 않음

### Legacy v1 Vercel Preview 실패 (기존 문제)
모든 PR에서 `Vercel` check가 실패 상태로 보입니다. 이는 v2 작업의 회귀가 아닙니다.

- legacy v1 `src/lib/supabase.ts`가 module 초기화 시점에 `NEXT_PUBLIC_SUPABASE_URL`을 요구합니다.
- Vercel 프로젝트에는 해당 변수가 Production 환경에만 등록되어 있어 Preview 빌드에서 값이 없습니다.
- `origin/main`을 그대로 체크아웃해 `pnpm install --frozen-lockfile` 후 `pnpm run build`를 실행하면 동일하게 `Error: supabaseUrl is required.`로 실패하는 것을 확인했습니다.
- 해결은 legacy v1 유지보수 또는 v1 Vercel 연동 종료 결정에 속하며 Stage 1~3 범위가 아닙니다.

### Stage 3
Draft PR #11: `feat/v2-architecture-foundation -> develop`

검증된 내용:
- React/Vite foundation
- Tauri 2 Windows native build smoke
- Rust/Axum API
- Rust workspace
- PostgreSQL 18 v2 migration
- `/health` / `/ready`
- generated OpenAPI
- rustfmt
- clippy `-D warnings`
- Rust tests
- TypeScript typecheck/build
- PostgreSQL migration + readiness

최신 확인한 `V2 Architecture` CI run `35555121713`은 모든 job 성공.

PR #11 최신 head는 이후 문서/CI 작업으로 변경될 수 있으므로, merge 전 실제 head와 최신 CI를 다시 확인합니다.

남은 Stage 3:
- generated pnpm/Cargo lockfile을 repository source of truth로 최종 커밋
- CI를 frozen/locked install로 전환
- 실제 개발 머신 `tauri dev`
- mobile target/plugin compatibility
- 운영/개발 config separation 재검토

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

## Production-first 데이터 정책

- production migration에 demo school/user/task/submission seed를 넣지 않음
- 저장소에 가상 fixture DB나 실제처럼 채운 demo dataset을 상시 보존하지 않음
- 테스트 실행 시 필요한 데이터만 생성하고 rollback/truncate/disposable DB 등으로 제거
- 실제 업무자료/개인정보는 public repository에 두지 않음
- 제품에 필수인 reference data는 demo data와 구분

## 아직 하지 않은 것

- PR #9/#11 merge
- Git history rewrite
- 외부 credential 실제 폐기/회전
- branch protection/ruleset 적용
- production infra 생성/배포
- ZITADEL/R2/NATS production 연결
- production DB migration
- Stage 5 UI 코드 구현
