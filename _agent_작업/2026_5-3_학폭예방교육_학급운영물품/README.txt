2026_5-3 양식 정리 결과

작업일: 2026-05-20 09:00:43 KST
원본 파일: /home/coseung2/.hermes/cache/documents/doc_f2497bc4ba56_2026_5-3.xlsx
Google Sheets: https://docs.google.com/spreadsheets/d/1vrJZ-P7_1bRs7Wp4PODnY-SygipOXSySqBWU7zSdG4o/edit?gid=0#gid=0
시트명: 2026 학폭예방교육 학급운영 물품

무슨 양식인가?
- 제목: 2026 학교폭력 예방교육 학급운영 물품 구입 목록
- 목적: 학교폭력 예방교육 학급운영 물품 구입 목록 수합
- 안내: * 학급당 10만원 학년별로 수합하여 하나의 파일로 부탁드려요^^                                                             (   )학년
- 기준: 학급당 10만원, 학년별 취합

엑셀 구조
- 입력 가능 물품 행: 30개
- 실제 입력되어 있던 물품 행: 6개
- 컬럼: 순번, 물품명, 예상단가, 수량, 예상금액, 할인가, 사는 곳, 상품번호/링크
- 예상금액은 엑셀에서 일부 수식으로 계산됨
- 할인가 컬럼은 원본에 존재하므로 보존함

정리 방식
현재 school-collect 앱은 반복 행 테이블을 직접 지원하지 않고, 일반 입력 항목만 지원합니다.
그래서 지금 바로 등록 가능한 방식으로 30개 물품 행을 고정 필드로 펼쳐 정리했습니다.

등록 필드 수
- 물품 30행 x 입력항목 7개 = 총 210개 필드
- 순번은 별도 입력칸으로 만들지 않고 물품01, 물품02처럼 필드명에 반영했습니다.

생성한 파일
1. template.json
   - 양식 제목, 설명, 연도, 원본 정보

2. fields.json
   - school-collect 현재 구조에 맞춘 210개 필드 목록

3. register-payload.json
   - /api/templates 에 POST할 수 있는 등록 payload

4. insert-template.sql
   - Supabase SQL Editor에서 바로 실행 가능한 템플릿 등록 SQL
   - 같은 제목/연도 템플릿이 있으면 필드를 새로 정리합니다.
   - 마감일이 없어 collect_runs는 만들지 않습니다.

5. seed-supabase-template.mjs
   - Supabase SDK로 템플릿을 직접 등록하는 스크립트
   - Prisma 사용 안 함

6. sample-export-layout.csv
   - 원본 엑셀의 현재 입력 예시와 30행 구조를 CSV로 정리한 파일

7. source-info.json
   - 원본 시트 정보, 병합 셀, 숨김 컬럼, 수식, 샘플 행 정보

8. future-table-field-proposal.json
   - 나중에 앱에 반복행 입력 UI를 만들 때 쓸 권장 구조
   - 현재 앱에는 바로 등록하지 않는 참고용입니다.

운영 메모
- Supabase 템플릿 등록 완료: ca8cddc5-3e4b-4e18-93f9-f177745d17aa
- 등록 필드 수 확인 완료: 210개
- 마감일이 아직 없어서 실제 수합 실행은 만들지 않았습니다.
- 앱의 '새 수합'에서 이 양식을 선택하고 마감일을 넣으면 실제 수합을 열 수 있습니다.
- 입력 단위는 학년/반입니다.
- 결과 CSV를 내려받으면 학년별 취합 파일로 정리하기 좋습니다.

주의
- 현재 앱 구조에서는 예상금액 자동계산이 안 됩니다. 입력자가 직접 적는 방식입니다.
- 반복행 UI를 만들면 이 양식은 훨씬 편해집니다. 그때는 future-table-field-proposal.json을 기준으로 바꾸면 됩니다.
