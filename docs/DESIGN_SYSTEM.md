# Figma Design System 계약

상태: **제품 시안 승인 완료**, 기본 코드 Design System/AppShell은 PR #17로 통합됐습니다. 후속 작업은 [TEAM_BACKLOG.md](TEAM_BACKLOG.md)의 C-01에서 관리합니다.

## Figma files

- `School Collect — Design System`
- `School Collect — Product`

Design System은 Foundations / Components / Patterns를, Product는 실제 업무 screen과 flow를 담당합니다.

## 핵심 원칙

### 1. Container보다 hierarchy
spacing, typography, divider로 해결 가능한 구분에 Card를 기본값으로 사용하지 않습니다.

### 2. Card = independent object
Card는 독립적으로 이해되는 정보 객체 또는 행동 객체일 때 사용합니다.

Card 후보:
- 독립적으로 해석 가능
- 전체/명확한 영역이 action object
- 고유 상태/긴급도/다음 행동 보유
- 배경 제거 시 실제 객체 경계가 모호해짐

### 3. Card sizing follows content/action density
카드 면적은 정보량과 행동 밀도에 비례합니다.

- 내용이 적으면 compact
- 정렬을 위한 빈 공간 최소화
- 기본 padding 14-16px 범위
- 읽기성/interaction target을 보존하는 최소 크기 우선
- width는 주변 alignment와 객체 중요도를 고려할 수 있으나 height/padding은 content density에 맞춤

### 4. Peer collection consistency
같은 peer collection에서 중요도만을 이유로 한 항목만 다른 container primitive로 바꾸지 않습니다.

긴급도는 semantic color, status, typography, badge로 표현합니다.

divider만으로 collection 경계가 약하면:
```text
List Surface
  ├─ List Row / Urgent
  ├─ List Row / Default
  ├─ List Row / Default
  └─ List Row / Default
```

을 사용하고 각 row를 개별 Card로 만들지 않습니다.

### 5. Measurable constraints
- radius <= 8px
- decorative shadow 없음
- 한 화면의 primary action hierarchy는 하나
- 8px spacing system
- neutral content surface + institutional blue accent
- active sidebar에 vertical indicator line 없음

## 현재 승인된 navigation 방향

Desktop:
- light blue-gray sidebar
- compact density
- selected item = light blue background + strong blue text
- vertical active indicator 없음
- content area = neutral white/gray

광범위한 blue wash, blue table band, blue filter strip은 사용하지 않습니다.

### 학교업무 / 개인기능 구분 계약

2026-09-27 작업 계획 기준입니다. 아래는 후속 UI 반영 계약이며 이번 문서 변경으로 Figma나 앱 화면을 변경한 것은 아닙니다.

- 학교 현황·자료수합·내 제출·구성원은 학교업무로 묶습니다. `내 제출`도 학교 membership이 필요합니다.
- 개인 바로가기·자동입력·감시는 개인기능으로 묶고 School Collect 로그인 없이 접근할 수 있게 합니다. 학교 API 장애로 개인 화면 전체를 막지 않습니다.
- 로그인 전에 학교 화면을 열면 Permission 상태에서 이유와 로그인 행동을 보여줍니다. 개인 화면으로 이동할 수 있어야 합니다.
- 설정은 기기/개인 도구 설정과 학교 계정/연결 설정을 구분합니다. 학교 로그아웃이 개인 버튼을 지우지 않도록 표시합니다.
- offline 상태는 영향을 받는 기능에 맞춰 표시합니다. 로컬 버튼 관리 가능 여부와 학교 제출/외부 사이트 연결 불가를 구분합니다.
- 두 영역 모두 기존 sidebar·token·List Surface/Row·상태 컴포넌트를 공유합니다. Figma 후속과 코드 반영 시 이 계약을 함께 대조합니다.

## Components

현재 Design System에서 정의/승인된 기준:
- Button
- FormField
- Status
- Card / Attention
- Card / Standalone Featured
- State panel: Empty / Loading / Error / Permission / Offline
- List Row: Default / Urgent
- List Surface
- Tabs reference
- flat Data Table reference

후속 C-01/Design System 보강 대상:
- Select
- Checkbox
- Radio
- Switch
- Dialog
- Sheet
- Toast
- detailed hover/pressed/focus-visible/keyboard states

## States

Screen은 default screenshot 하나가 아니라 state set으로 설계합니다.

기본 검토:
- Default
- Loading
- Empty
- Error
- Permission
- Offline

로그인 없이 쓸 수 있는 화면(로컬 도구)과 학교 멤버십이 필요한 화면을 구분합니다. 멤버십이 필요한 화면을 로그인 전에 열면 Permission 상태로 이유와 로그인 action 하나를 보여주고, 로컬 화면은 로그인 없이 그대로 동작합니다.

업무 flow에서는 추가로 확인:
- data = 0
- slow network
- disconnected network
- no permission
- duplicate click/request
- destructive confirmation/recovery
- concurrent update/conflict

## Figma -> code contract

```text
Figma semantic variable
        ↓
repository token
        ↓
packages/ui
        ↓
Tauri product UI
```

제품 코드에서 임의 hex/spacing/radius를 반복하지 않습니다. reusable visual decision이 Product에서 새로 생기면 Design System 반영 여부를 동시에 판단합니다.

Figma와 코드가 충돌할 경우 조용히 한쪽을 덮어쓰지 않고 차이를 기록하고 승인 기준을 정합니다.

## Accessibility

각 UI 변경에서 실제 코드와 함께 확인:
- keyboard navigation
- focus-visible
- semantic roles/name
- contrast
- zoom/reflow
- touch target where applicable
- 한국어 긴 label
- narrow desktop window
- destructive action confirmation/recovery

## 승인과 남은 작업

제품 시안 승인과 기본 `packages/ui` token/component 및 AppShell 구현은 완료됐습니다.

남은 것은 “다시 시안 만들기”가 아니라:
- Figma variables -> repo token mapping의 세부 대조와 누락 보강
- 추가 component와 학교업무/개인기능 진입·설정·상태 구분
- interaction/accessibility detail
- Desktop/Mobile density mapping
- Library publish/Code Connect 적용 여부
- mobile shell 재검증

입니다.
