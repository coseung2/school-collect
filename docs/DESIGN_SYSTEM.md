# Figma Design System 계약

상태: **제품 시안 승인 완료**. 코드 Design System 구현은 Stage 5에서 진행합니다.

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

## Components

현재 Design System에서 정의/승인된 기준:
- Button
- FormField
- Status
- Card / Attention
- Card / Standalone Featured
- List Row: Default / Urgent
- List Surface
- Tabs reference
- flat Data Table reference

후속 Stage 5/Design System 보강 대상:
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

Stage 5에서 실제 코드와 함께 확인:
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

제품 시안은 승인되었습니다.

남은 것은 “다시 시안 만들기”가 아니라:
- Figma variables -> repo token mapping
- `packages/ui` 구현
- interaction/accessibility detail
- Desktop/Mobile density mapping
- Library publish/Code Connect 적용 여부
- mobile shell 재검증

입니다.
