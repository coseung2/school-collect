2026 도서관 보관 도서구입 신청 목록 정리 결과

작업일: 2026-05-20 09:16:09 KST
원본 링크: https://docs.google.com/spreadsheets/d/1vrJZ-P7_1bRs7Wp4PODnY-SygipOXSySqBWU7zSdG4o/edit?gid=0#gid=0
다운로드 파일: source_google_sheet.xlsx, source_google_sheet.csv

무슨 양식인가?
- 제목: 2026학년도 1차 도서관 보관 도서구입 신청 목록(장량초등학교)
- 목적: 도서관 보관 도서구입 신청 목록 수합
- 컬럼: 순서, 도서명, 저자, 출판사, 정가, 수량, 금액, 비고
- 입력 가능 행: 120행
- 현재 입력된 도서 행: 43개
- 총 권수: 44권
- 총 금액: 829,700원

생성한 파일
1. README.txt
   - 이 작업 요약

2. source_google_sheet.csv
   - Google Sheet에서 내려받은 CSV 원본

3. source_google_sheet.xlsx
   - Google Sheet에서 내려받은 XLSX 원본

4. source-info.json
   - 링크, 제목, 컬럼, 행 수, 총권수, 총금액, 비고 분류 요약

5. books-normalized.json
   - 도서 목록을 웹앱에서 쓰기 좋은 JSON 구조로 정리한 파일

6. sample-import-layout.csv
   - 웹앱 표 입력/가져오기 기준 CSV

7. register-payload-current-compatible.json
   - 현재 앱 구조에서 바로 쓸 수 있는 임시 붙여넣기형 양식 payload

8. insert-current-compatible-template.sql
   - 임시 붙여넣기형 양식을 Supabase SQL Editor에 등록하는 SQL

9. recommended-table-template.json
   - 정식 구현 후 등록할 추천 테이블형 양식 구조

10. alter-field-type-table.sql
   - table field_type을 허용하는 Supabase SQL

11. WEBAPP_IMPLEMENTATION_PLAN.md
   - 웹앱에 반복행 테이블 입력을 어떻게 구현하면 좋은지 정리한 계획서

판단
- 이 양식은 일반 입력칸 여러 개가 아니라 반복행 표 양식입니다.
- 현재 앱 구조에 억지로 넣으면 최대 120행 x 7항목 = 840개 입력칸이 생겨서 실제 사용성이 나쁩니다.
- 그래서 정식 템플릿 등록은 하지 않고, 먼저 반복행 table 필드를 구현하는 쪽으로 정리했습니다.
- 급하게 운영해야 할 때만 임시 붙여넣기형 양식을 쓰면 됩니다.

웹앱 추천 구현
- 먼저 API 응답의 snake_case/camelCase 매핑 정리
- form_fields.field_type에 table 추가
- options JSONB에 컬럼 정의 저장
- collect_answers.value에 JSON 배열 저장
- 입력 화면에 TableField 컴포넌트 추가
- 정가 x 수량 = 금액 자동계산
- Google Sheet 복사 붙여넣기 지원
- 결과 CSV는 행 단위로 펼쳐 다운로드
