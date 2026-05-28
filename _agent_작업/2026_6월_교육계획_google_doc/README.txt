Google Docs 분석 결과 — 2026학년도 6월 교육 계획

원문 링크:
https://docs.google.com/document/d/15HeWnTlPU7DRABfDxITW9wqtSxaFvIiFlj1fzy_jPMc/edit?usp=sharing

요약:
- 문서 성격: 월중 교육 계획 수합표
- 표 구조: 주차 / 날짜 / 추진 내용
- 날짜 행: 22개
- 실제 추진 항목: 24개
- 빈 입력 후보 행: 9개
- 제목에 4월/6월이 같이 남아 있어 실제 운영 전 제목 정리 필요

판단:
이 문서는 단순 텍스트 입력 양식이 아니라, 교직원별로 일정 항목을 여러 개 입력하고 담당자가 검토/확정한 뒤 학교 구성원 모두가 읽기전용 월중계획으로 보는 구조가 맞습니다.

추천 구현:
- 입력자: 자기 일정만 행 추가 방식으로 입력
- 담당자: 전체 일정을 날짜순/주차별로 검토하고 최종 월중계획 확정
- 전체 구성원: 확정된 월중계획을 읽기전용 파일/페이지로 확인
- 필요한 기능: table field_type, 교직원/부서 제출 식별자, 담당자 확정 화면, 전체 공개 열람 화면, CSV/엑셀/PDF 또는 링크 공유

중요:
현재 앱 구조에 바로 등록하려면 textarea 임시안은 가능하지만, 정식 운영은 비추천입니다.
도서구입 양식과 같이 table 필드 구현 후 등록하는 것이 좋습니다.

주요 파일:
- WEBAPP_IMPLEMENTATION_ANALYSIS.md
- parsed-schedule.json
- parsed-schedule.csv
- event-items.csv
- recommended-monthly-plan-template.json
- register-payload-current-compatible.json
- sample-export-layout.csv
- schema-extension-staff-submissions.sql
