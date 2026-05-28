2026학년도 1차 도서관 보관 도서구입 신청 목록 구현 분석
============================================================

대상 양식
--------
- 원본 제목: 2026학년도 1차 도서관 보관 도서구입 신청 목록(장량초등학교)
- 확인된 CSV 구조: 122행, 컬럼은 순서 / 도서명 / 저자 / 출판사 / 정가 / 수량 / 금액 / 비고
- 성격: 한 제출자가 여러 도서 행을 반복 입력하고, 각 행의 정가·수량·금액을 계산/취합해야 하는 반복행 테이블형 양식

현재 school-collect 구조 요약
---------------------------
- Next.js 16 + Supabase SDK 기반, Prisma 없음
- DB: form_templates / form_fields / collect_runs / collect_submissions / collect_answers
- form_fields.field_type 허용값: text, number, date, select, textarea
- collect_answers는 submission_id + field_id당 value TEXT 1개만 저장
- 현재 입력 UI는 필드 목록을 세로로 렌더링하며 반복행/테이블 입력을 직접 지원하지 않음
- 결과 UI는 제출 1건을 1행으로 보고, 각 form_field를 열로 펼침

핵심 결론
---------
이 양식은 현재 구조를 그대로 쓰면 사용성과 데이터 품질이 크게 나빠진다.
단기 임시 수합이 급하면 textarea 1개에 CSV/붙여넣기 방식으로 받을 수는 있지만, 웹앱의 재사용 가치와 결과 취합 품질을 생각하면 field_type='table' 또는 유사한 반복행 필드 타입을 추가하는 편이 맞다.

비교 1. 현재 구조를 무리하게 쓰는 방안
------------------------------------

방안 A: textarea 1개에 전체 도서 목록 붙여넣기
- 템플릿 필드 예: "도서구입 신청 목록" textarea 1개
- 안내문에 "도서명,저자,출판사,정가,수량,금액,비고 순서로 줄마다 입력" 같은 규칙을 둠

장점
- DB 변경 없음
- API 변경 거의 없음
- 현재 UI에서 바로 생성 가능
- 가장 빠르게 임시 운영 가능

단점
- 사용자가 쉼표/탭/줄바꿈 형식을 틀리기 쉬움
- 금액 자동 계산 불가 또는 매우 제한적
- 행별 검증(정가 숫자, 수량 숫자, 금액=정가*수량) 불가
- 결과 화면/CSV가 한 셀에 뭉쳐져 나와 후처리 필요
- 원본 Google Sheet의 행/열 형태를 웹앱이 보존하지 못함
- 향후 다른 반복행 양식(물품 구입, 학생 명단, 체험학습 명단 등)에도 재사용성이 낮음

방안 B: 고정 행 수만큼 필드를 전부 생성
- 예: 1행_도서명, 1행_저자, ..., 122행_비고
- 122행 x 7개 컬럼 = 약 854개 field 생성

장점
- collect_answers 모델을 그대로 유지하면서 결과 CSV를 열 단위로 만들 수 있음
- 행/열 값을 개별 필드로 저장하므로 textarea보다는 검증 여지가 있음

단점
- 필드 수가 비정상적으로 많아 템플릿 생성/수정/렌더링이 무거움
- 입력 화면이 매우 길고 불편함
- 빈 행이 대부분이어도 답변/필드가 과도하게 많음
- 결과 화면이 854개 열이 되어 사실상 사용 불가
- 원본처럼 "도서 한 권 = 결과 한 행"으로 취합하려면 별도 후처리가 필요
- 122행 제한이 하드코딩되고, 다른 양식에는 또 다른 필드 폭증 발생

방안 C: 도서 1권을 제출 1건으로 취급
- 현재 collect_submissions UNIQUE(run_id, grade, class_num) 때문에 같은 학급이 여러 행을 제출하기 어렵다.
- unique 제약을 바꾸지 않으면 구조적으로 맞지 않고, 바꾸더라도 "제출"의 의미가 깨진다.

판단
- 단 1회성 임시 운영: textarea 붙여넣기 가능
- 실제 웹앱 기능으로는 비추천
- 특히 이 양식은 122행 반복 입력과 금액 계산이 중요하므로 반복행 테이블 UI가 자연스럽다.

비교 2. 반복행 테이블 UI를 추가하는 방안
--------------------------------------

권장 모델
- form_fields.field_type에 'table' 추가
- form_fields.options JSONB에 테이블 컬럼 정의 저장
- collect_answers.value에 해당 테이블의 행 배열(JSON)을 저장하는 MVP부터 시작

예시 field options
------------------
{
  "minRows": 1,
  "maxRows": 122,
  "columns": [
    { "key": "order", "label": "순서", "type": "number", "readonly": true, "autoIndex": true },
    { "key": "title", "label": "도서명", "type": "text", "required": true },
    { "key": "author", "label": "저자", "type": "text" },
    { "key": "publisher", "label": "출판사", "type": "text" },
    { "key": "listPrice", "label": "정가", "type": "number", "min": 0 },
    { "key": "quantity", "label": "수량", "type": "number", "min": 1, "default": 1 },
    { "key": "amount", "label": "금액", "type": "number", "formula": "listPrice * quantity", "readonly": true },
    { "key": "note", "label": "비고", "type": "text" }
  ]
}

저장 value 예시
---------------
[
  { "order": 1, "title": "...", "author": "...", "publisher": "...", "listPrice": 12000, "quantity": 1, "amount": 12000, "note": "" },
  { "order": 2, "title": "...", "author": "...", "publisher": "...", "listPrice": 15000, "quantity": 2, "amount": 30000, "note": "" }
]

장점
- 원본 Google Sheet 구조와 가장 유사한 입력 경험 제공
- 도서 행 추가/삭제가 자연스러움
- 정가·수량·금액 자동 계산 가능
- 숫자/필수값/최대 122행 등 검증 가능
- 결과 CSV를 원본과 같은 컬럼 구조로 평탄화 가능
- 다른 반복행 양식에도 재사용 가능
- 기존 form_templates/form_fields/collect_submissions/collect_answers 구조를 크게 버리지 않아도 됨

단점
- field_type CHECK 변경 필요
- FormField 타입, 관리자 템플릿 생성 UI, 입력 UI, 제출 API 검증, 결과 UI/CSV 로직 변경 필요
- collect_answers.value에 JSON 문자열을 넣는 MVP는 DB 차원의 행/셀 검색·집계가 약함
- 나중에 행 단위 검색/집계/부분 수정이 중요해지면 정규화 테이블로 확장해야 함

필요한 변경 사항
---------------

1) DB 변경 - MVP
- form_fields.field_type CHECK에 'table' 추가
- form_fields.options JSONB는 이미 있으므로 컬럼 정의 저장에 활용 가능
- collect_answers.value TEXT에 JSON.stringify(rows)를 저장
- Prisma 사용 금지. Supabase SQL Editor 또는 Supabase SDK만 사용

예상 SQL 방향
- ALTER TABLE form_fields DROP CONSTRAINT ...;
- ALTER TABLE form_fields ADD CONSTRAINT ... CHECK (field_type IN ('text','number','date','select','textarea','table'));
- 실제 constraint 이름은 Supabase에서 확인 후 적용 필요

2) DB 변경 - 장기 확장안
- JSON 문자열 저장으로 충분하지 않다면 다음 중 하나 선택
  a. collect_answers에 value_json JSONB 컬럼 추가
  b. collect_answer_rows 테이블 추가: id, answer_id/submission_id/field_id, row_order, data JSONB
  c. collect_answer_cells까지 완전 정규화
- 현재 규모와 개발 속도 기준으로는 a 또는 현재 value TEXT JSON 저장이 현실적이다.

3) API 변경
- /api/templates POST
  - fieldType='table' 허용
  - options가 문자열뿐 아니라 객체(JSON)여도 저장되도록 정리
  - table field의 columns 유효성 검사 필요
- /api/submissions POST
  - table answer는 value가 배열인지 확인
  - maxRows, required columns, number columns 검증
  - amount 같은 formula 필드는 서버에서도 재계산하거나 최소 검증
  - 빈 행 제거 정책 필요: 도서명/저자/출판사/가격 등이 모두 빈 행은 저장 제외
- /api/submissions GET
  - table value를 JSON.parse해서 내려줄지, 기존 value 문자열을 유지하고 클라이언트에서 파싱할지 결정
  - 권장: API 응답에서 table 필드만 parsedValue를 추가하거나 results 페이지에서 안전 파싱

4) UI 변경 - 입력 화면
- src/app/collect/[id]/page.tsx에 TableField 컴포넌트 추가 권장
- 기능
  - 행 추가/삭제
  - 순서 자동 번호
  - 정가·수량 입력 시 금액 자동 계산
  - 122행 제한
  - 빈 행 자동 무시 또는 제출 전 정리
  - 모바일에서는 가로 스크롤 또는 카드형 행 편집 제공
  - Google Sheet/Excel에서 복사한 TSV 붙여넣기 지원을 넣으면 실사용성이 크게 좋아짐

5) UI 변경 - 관리자 템플릿 생성
- 필드 타입 목록에 'table' 추가
- 단순 MVP에서는 "도서구입 신청 목록" 같은 프리셋 템플릿을 선택하게 하는 방식 추천
- 처음부터 임의 컬럼 편집기를 만들면 범위가 커진다.
- 프리셋: 도서구입 목록, 물품구입 목록, 학생명단 등으로 확장 가능

6) UI 변경 - 결과/CSV
- 현재 결과 화면은 제출 1건 = 1행, field = 열 구조다.
- table 필드는 두 가지 보기 필요
  a. 제출자별 요약: 학년-반, 제출자, 도서 건수, 총 금액
  b. 상세 펼침/다운로드: 학년-반, 제출자, 순서, 도서명, 저자, 출판사, 정가, 수량, 금액, 비고
- CSV 다운로드는 상세 평탄화가 필요하다.
- 도서구입 양식의 최종 CSV 컬럼 권장:
  학년, 반, 제출자, 순서, 도서명, 저자, 출판사, 정가, 수량, 금액, 비고, 제출일

단계별 권장안
-------------

0단계: 지금 당장 정리
- 이 양식은 "반복행 테이블형"으로 분류한다.
- 현재 기본 필드(text/number/date/select/textarea)만으로는 정식 구현 대상이 아니라고 명확히 기록한다.

1단계: 임시 운영이 급한 경우
- textarea 1개짜리 임시 템플릿을 만든다.
- 안내문에 탭/줄바꿈 형식과 예시를 제공한다.
- 결과는 수동 후처리 전제로 둔다.
- 이 방식은 단기 임시안으로만 사용하고, 정식 기능으로 고착시키지 않는다.

2단계: 추천 MVP 구현
- field_type='table' 추가
- collect_answers.value에 JSON 배열 저장
- 도서구입 목록 프리셋 1개를 우선 구현
- 입력 화면에 TableField 컴포넌트 추가
- 제출 API에서 JSON 배열 검증
- 결과 CSV에서 table rows를 평탄화

3단계: 사용성 강화
- Excel/Google Sheet 붙여넣기 지원
- 합계 표시: 총 권수, 총 금액
- 행별 오류 표시
- 빈 행 자동 정리
- 122행까지 빠르게 입력 가능한 키보드 UX 개선

4단계: 범용 테이블 양식화
- 관리자에서 테이블 컬럼을 직접 편집할 수 있게 함
- number/text/select/textarea/date 컬럼 지원
- formula, readonly, required, default, min/max, width 같은 column options 지원
- 도서구입 외 반복행 업무에도 재사용

5단계: 데이터 분석/검색이 중요해진 뒤 정규화
- 단순 JSON 저장으로 한계가 생기면 collect_answer_rows 또는 value_json JSONB로 이전
- 행 단위 검색, 품목별 집계, 가격 합계 쿼리 등을 DB에서 처리

최종 추천
---------
- 추천: 반복행 테이블 UI 추가
- MVP 저장 방식: collect_answers.value에 JSON 배열 저장
- MVP DB 변경: field_type CHECK에 table만 추가
- MVP UI 범위: 도서구입 목록 프리셋 + TableField + 결과 CSV 평탄화
- 현재 구조 강행은 임시 textarea 입력 외에는 비추천

주의 사항
---------
- 원본 Google Sheet를 덮어쓰지 않는다.
- Prisma를 도입하지 않는다.
- Supabase SDK/SQL 기반으로만 변경한다.
- DB 스키마 변경 전에는 실제 constraint 이름을 확인해야 한다.
- table value는 클라이언트 계산만 믿지 말고 제출 API에서도 금액 계산/검증을 수행하는 편이 안전하다.
