# Stage 5 code migration plan

기준일: 2026-09-22

이 문서는 승인된 `School Collect — Design System` / `School Collect — Product`
기준을 `packages/ui`와 `apps/app`에 옮기는 첫 번째 코드 배치의 범위를 기록합니다.
실제 업무 데이터나 production seed는 이 단계에 추가하지 않습니다.

## READ

- 디자인 기준: [DESIGN_SYSTEM.md](DESIGN_SYSTEM.md)의 radius, spacing, sidebar,
  card, list surface, state 계약.
- 코드 기준: Stage 3의 `apps/app/src/App.tsx`는 API health 진단 화면 하나이며,
  공유 token이나 component package가 없었습니다.
- 현재 API 계약: 클라이언트가 호출할 수 있는 검증된 endpoint는 `/health`이며,
  PostgreSQL에는 직접 연결하지 않습니다.

## CODE COMPARE → mapping

| 승인 기준 | 코드 destination | 검증 방법 |
| --- | --- | --- |
| semantic color / spacing / radius | `packages/ui/src/styles.css` CSS custom properties | raw color와 spacing 반복 검색, typecheck/build |
| Button, Status, Card | `packages/ui/src/components.tsx` | keyboard focus와 disabled/loading 상태 확인 |
| List Surface / List Row | `packages/ui/src/components.tsx` | peer row를 동일 primitive로 렌더링 |
| Sidebar / Header / Tabs / Table | `packages/ui/src/components.tsx` | wide/narrow layout 확인 |
| Empty / Loading / Error / Permission / Offline | state components in `packages/ui` | state-specific icon, copy, recovery action 확인 |
| AppShell | `apps/app/src/App.tsx` | navigation, API health, offline event 확인 |

## MIGRATION PLAN

### Desktop Home alignment (2026-09-23)

- Source: Product `Desktop / Home` node `13:2` (1280×820), read through Figma design context. Preserve this approved frame; no Figma write.
- Classification: the current four-item icon navigation and diagnostic-first overview diverge from the approved grouped, text-first sidebar and work-first hierarchy. The depicted counts, named work, search, account identity, and actions lack an implemented data/auth contract; do not copy them as live data or make inert controls.
- Destination: `packages/ui` Sidebar/Header tokens and `apps/app` route composition. Keep `/health` diagnostic in Settings. Use an empty work state on Home until authorized API data exists.
- Rollback: revert this code batch; no DB, API, Figma node, or native capability changes.
- Screenshot targets: 1280×820 and 720×700 Home empty, Settings checking/error, keyboard and offline state. Compare shell geometry and hierarchy with node `13:2`, not sample business values.

1. `packages/ui`를 React peer dependency만 사용하는 shared package로 추가합니다.
2. 승인된 semantic token을 CSS custom property로 고정하고, 제품 화면은 token을
   통해서만 색·spacing·radius를 사용합니다.
3. AppShell은 실제로 동작하는 API health 확인과 빈 상태만 표시합니다. 서버에
   존재하지 않는 업무 목록이나 demo school/user/task/submission을 만들지 않습니다.
4. desktop compact sidebar를 기본으로 하고, 좁은 창에서는 navigation을
   horizontal flow로 재구성합니다.
5. 이 배치는 Figma 파일의 canonical node를 변경하지 않는 code-only migration입니다.
   저장소에는 Figma node ID가 없으므로 Figma write는 실행하지 않으며, 승인된 원본을
   덮어쓰지 않습니다.

## State and screenshot targets

- wide desktop: 1280×820, sidebar + header + overview
- narrow desktop: 1024×700, content remains reachable without horizontal page overflow
- API checking / healthy / error
- browser offline banner and recovery action
- empty collection state
- keyboard focus-visible and reduced-motion behavior

## Rollback

이 배치는 DB schema, API contract, Tauri capability, 또는 production configuration을
변경하지 않습니다. 문제가 생기면 feature branch의 commit을 revert하고 `apps/app`의
package import와 stylesheet import를 이전 상태로 되돌리면 됩니다.
