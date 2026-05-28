# 도서구입 신청 목록 웹앱 구현 계획

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Google Sheet/Excel의 도서구입 신청 목록을 선생님이 웹앱에서 표처럼 입력하고, 담당자가 학급/학년별로 바로 취합·CSV 다운로드할 수 있게 만든다.

**Architecture:** 기존 form_templates/form_fields 구조는 유지하되, form_fields.field_type에 table을 추가한다. 테이블 입력값은 MVP 단계에서는 collect_answers.value에 JSON 문자열로 저장하고, 결과 화면에서 행 단위로 펼쳐 보여준다. 나중에 사용량이 늘면 JSONB 컬럼 또는 별도 행 테이블로 확장한다.

**Tech Stack:** Next.js 16 App Router, Supabase JS SDK, PostgreSQL, React client component.

---

## 왜 현재 방식 그대로 넣으면 안 좋은가

현재 앱은 한 양식에 고정 입력칸을 여러 개 만드는 구조다. 이 시트는 제목/헤더를 제외하면 입력 가능 슬롯이 120행이고, 실제 입력된 도서는 43개다. 양식을 그대로 펼치면 최대 120행 x 7항목 = 840개 입력칸이 된다.

문제:
- 입력 화면이 너무 길어서 선생님이 쓰기 어렵다.
- 금액 자동계산이 안 된다.
- 행 추가/삭제가 어렵다.
- 결과 CSV가 가로로 너무 길어진다.
- Google Sheet에서 복사해 붙여넣는 운영이 어렵다.

결론: 이 양식은 일반 폼이 아니라 반복행 테이블로 구현해야 한다.

## 확정 UX 방향: 선생님별 입력, 담당자 전체 누적

이 기능의 핵심은 120행짜리 원본 시트를 선생님에게 그대로 보여주는 것이 아니다.

### 선생님 입력 화면

- 선생님은 자기 학년/반/이름 기준으로만 입력한다.
- 처음에는 빈 도서 목록 또는 1행만 보인다.
- 필요한 만큼 `행 추가`로 도서를 늘린다.
- 예: 어떤 선생님이 5권만 신청하면 그 선생님 화면에는 5줄만 있으면 된다.
- 120행 고정 입력칸은 선생님 화면에 노출하지 않는다.

### 담당자 취합 화면

- 담당자는 모든 선생님 제출분을 하나의 누적표로 본다.
- 제출자별/학년별/전체 보기로 필터링할 수 있다.
- 전체 총권수와 총금액을 자동 합산한다.
- CSV/엑셀 내보내기는 `학년, 반, 제출자, 순서, 도서명, 저자, 출판사, 정가, 수량, 금액, 비고`처럼 행 단위로 펼친다.
- 제출/미제출 현황도 함께 확인할 수 있게 한다.

즉 데이터 구조는 `선생님별 제출 1건 안에 도서 행 여러 개`이고, 담당자 화면에서만 이 도서 행들을 전체 누적으로 펼쳐 보여준다.

## 추천 구현

### 0단계: Supabase 응답 매핑 먼저 정리

현재 DB 컬럼은 snake_case이고 프론트 타입은 camelCase다.
예: `field_type` ↔ `fieldType`, `template_id` ↔ `templateId`, `target_type` ↔ `targetType`, `submitted_at` ↔ `submittedAt`.

반복행 테이블을 넣기 전에 API route에서 응답을 camelCase로 정리해야 입력/결과 화면이 안정적으로 동작한다.

수정 파일:
- `src/lib/mappers.ts` 생성
- `src/app/api/templates/route.ts`
- `src/app/api/runs/route.ts`
- `src/app/api/submissions/route.ts`

### 1단계: field_type='table' 추가

수정 파일:
- `supabase-schema.sql`
- `src/lib/types.ts`
- `src/app/admin/new/page.tsx`
- `src/app/collect/[id]/page.tsx`
- `src/app/collect/[id]/results/page.tsx`

DB 변경:
- form_fields.field_type CHECK에 `table` 추가
- options JSONB에 컬럼 정의 저장

### 2단계: TableField 입력 컴포넌트 추가

생성 파일:
- `src/components/TableField.tsx`

기능:
- 행 추가
- 행 삭제
- 순서 자동번호
- 정가 x 수량 = 금액 자동계산
- 총권수/총금액 표시
- Google Sheet에서 복사한 표 붙여넣기 지원

### 3단계: 제출 저장 방식

기존 구조 유지:
- collect_answers.value에 JSON 문자열 저장

예시:
```json
[
  {"order":1,"title":"브로큰 컨트리","author":"클레어 레슬리 홀","publisher":"북로망스","listPrice":19500,"quantity":1,"amount":19500,"note":"성인용"}
]
```

### 4단계: 결과 화면 개선

결과 화면에서 table 필드는 두 방식으로 보여준다.
- 제출자별 요약: 총권수, 총금액
- 상세 펼침: 도서명/저자/출판사/정가/수량/금액/비고 행 목록

CSV 다운로드는 행 단위로 평탄화한다.

권장 CSV 컬럼:
학년, 반, 제출자, 순서, 도서명, 저자, 출판사, 정가, 수량, 금액, 비고

### 5단계: 양식 등록 방식

이 양식은 `recommended-table-template.json` 기준으로 등록한다.
단, 앱 코드에 table 타입이 들어가기 전에는 등록하지 않는다.

## 단계별 작업

### Task 0: API 응답 camelCase 매핑 정리

파일:
- Create: `src/lib/mappers.ts`
- Modify: `src/app/api/templates/route.ts`
- Modify: `src/app/api/runs/route.ts`
- Modify: `src/app/api/submissions/route.ts`

작업:
- DB 응답의 `field_type`, `template_id`, `target_type`, `target_value`, `class_num`, `submitted_at`, `field_id`를 프론트가 쓰는 camelCase로 변환한다.
- form_fields 배열도 `order` 기준으로 정렬해서 내려준다.

검증:
- 기존 text/number/textarea 필드가 입력 화면에 정상 렌더링되는지 확인
- `rtk npm run lint`

### Task 1: table 타입을 타입 정의에 추가

파일:
- `src/lib/types.ts`

작업:
- FormField.fieldType union에 `table` 추가
- fieldTypeLabel에 `table: '표'` 추가

검증:
- `rtk npm run lint`

### Task 2: DB 스키마에 table 허용

파일:
- `supabase-schema.sql`
- 새 마이그레이션 문서 또는 `_agent_작업/.../alter-field-type-table.sql`

SQL:
```sql
ALTER TABLE form_fields DROP CONSTRAINT IF EXISTS form_fields_field_type_check;
ALTER TABLE form_fields ADD CONSTRAINT form_fields_field_type_check CHECK (field_type IN ('text','number','date','select','textarea','table'));
```

검증:
- Supabase SQL Editor에서 실행 후 table 타입 insert 테스트

### Task 3: TableField 컴포넌트 생성

파일:
- `src/components/TableField.tsx`

작업:
- columns 옵션을 받아 표 렌더링
- 행 추가/삭제
- 자동번호
- 금액 계산
- 붙여넣기 파싱

검증:
- 도서명/저자/출판사/정가/수량/금액/비고가 한 행으로 입력되는지 확인
- 정가 19500, 수량 2 입력 시 금액 39000 확인

### Task 4: collect 입력 화면에 table 렌더링 추가

파일:
- `src/app/collect/[id]/page.tsx`

작업:
- field.fieldType === 'table' 분기 추가
- TableField value를 JSON string으로 answers에 저장

검증:
- table 필드 제출 후 collect_answers.value에 JSON 배열 문자열 저장 확인

### Task 5: 결과 화면에서 table 답변 펼치기

파일:
- `src/app/collect/[id]/results/page.tsx`

작업:
- table 필드는 일반 셀에 JSON을 그대로 보여주지 않음
- 제출별 총권수/총금액 요약 표시
- CSV 다운로드 시 행 단위로 펼침

검증:
- CSV 컬럼이 학년, 반, 제출자, 순서, 도서명, 저자, 출판사, 정가, 수량, 금액, 비고 순서로 내려받아지는지 확인

### Task 6: 도서구입 추천 템플릿 등록

파일:
- 이 폴더의 `recommended-table-template.json`

작업:
- table 타입 배포 후 이 JSON 기준으로 템플릿 등록

검증:
- 앱에서 양식 선택 시 '도서 목록' 표가 뜨는지 확인

## 임시 운영안

정식 구현 전 급하게 수합해야 한다면 `register-payload-current-compatible.json` 또는 `insert-current-compatible-template.sql`을 사용한다.
이 방식은 도서 목록 전체를 textarea에 붙여넣는 방식이라 편의성은 떨어지지만, 현재 앱 구조에서는 바로 가능하다.

## 최종 추천

바로 구현한다면 Task 1~5를 먼저 한다. 이 기능이 들어가면 도서구입뿐 아니라 물품구입, 준비물, 행사목록처럼 행이 반복되는 모든 학교 수합에 재사용할 수 있다.
