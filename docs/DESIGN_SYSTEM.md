# Figma 디자인 시스템 계약

상태: 생성/승인 전 명세. 연결된 Figma 팀 선택이 필요하며 실제 Figma 파일과 Library는 아직 생성되지 않았습니다.

## 파일과 책임

`School Collect — Design System`과 `School Collect — Product`를 분리합니다. Design System에는 Foundation/Components/Patterns/Playground/Deprecated를, Product에는 Desktop/Mobile 및 Collect 흐름을 둡니다. 최종 스타일·컬러·타이포·density·주요 상태는 헤드가 승인합니다.

## 토큰

Reference -> semantic -> component alias 순서입니다. Figma의 `sys/color/action/primary`는 저장소 manifest의 같은 의미 이름, CSS `--color-action-primary`와 대응합니다. 제품 코드에서 임의 hex/여백을 반복하지 않습니다.

Theme의 Light/Dark와 Density의 Compact/Comfortable은 별도 축으로 정의합니다. 실제 mode/library 사용 가능 여부는 선택한 팀 권한에서 확인합니다. 모바일은 touch target, safe-area, 키보드, back navigation이 별도 검수 대상입니다.

최소 토큰: surface/background, text primary/secondary/disabled, border/focus, action, success/warning/danger/info, font size/weight/line-height, spacing, sizing, radius, elevation, motion, z-index.

## 기준 컴포넌트

Button/IconButton, Input/Textarea/Select, Checkbox/Radio/Switch, FormField, Badge/Alert, Dialog/Sheet, Tabs, Table, Empty/Loading/Error/Offline 상태를 우선합니다. TeacherPicker, DeadlineBadge, SubmissionStatus, FormRenderer는 제품 도메인 컴포넌트이며 UI primitive와 분리합니다.

컴포넌트마다 default/hover/focus/pressed/disabled/loading/error를 필요한 범위에서 정의합니다. 모든 기능을 hover/right-click만으로 제공하지 않습니다. 한국어 긴 라벨, 좁은 창, 키보드-only, 확대, 터치 입력을 검수합니다.

## Figma와 코드 변경 절차

1. Figma 변경 제안과 node URL/의도를 기록합니다.
2. token/variant 이름과 소비 코드 영향, 접근성을 검토합니다.
3. 승인된 manifest 변경을 Git PR로 제출합니다.
4. generated CSS와 UI component/catelog 검증을 함께 수행합니다.
5. 코드 릴리스와 Figma Library 변경 기록을 연결합니다.

Figma는 디자인 의도/시안의 기준, 저장소의 승인된 manifest는 배포되는 token 값의 기준입니다. 양쪽을 독립적으로 변경하지 않습니다. 자동 동기화·Code Connect·Library publish가 실제 설정되지 않았으면 수동 대응표로 관리하고 자동이라고 부르지 않습니다.

## 완료 증거

실제 파일 URL, variables/components 및 variant 목록, Desktop/Mobile Shell 캡처, Collect 핵심 화면의 상태별 검수, token manifest 대응, 헤드 승인 기록. 이 문서만으로 4단계를 완료 처리하지 않습니다.
