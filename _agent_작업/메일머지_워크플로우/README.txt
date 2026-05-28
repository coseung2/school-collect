HWPX 메일머지 워크플로우 (UTF-8 BOM)
======================================

폴더 구조:
  README.md         — 전체 워크플로우 설명
  mailmerge.py      — HWPX 템플릿 + CSV 데이터 → 개인별 HWPX 생성
  update_csv.py     — OCR 결과 CSV 수정 도구
  batch_update.py   — 여러 수정 건 한 번에 반영
  의존성.txt      — 필요 패키지 (lxml)
  템플릿_관리.md     — 템플릿 라이브러리 관리 방법
  연간_가정통신문_자동화_계획.md — 연간 자동화 로드맵

사용 순서:
  1. 인박스/에 PDF + HWPX 템플릿 투입
  2. 에이전트가 GPT-5.5 Vision으로 OCR (delegate_task)
  3. ocr_결과.csv 를 선생님에게 검증 요청
  4. update_csv.py 로 수정 반영
  5. python mailmerge.py 템플릿.hwpx ocr_결과.csv 출력폴더
  6. 출력물 검증

문의: 이 워크플로우에 대한 질문은 에이전트에게
