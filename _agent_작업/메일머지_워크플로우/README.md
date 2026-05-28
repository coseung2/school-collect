# HWPX 메일머지 워크플로우 — 신체검사 → 가정통신문

## 1회 실행 시간: 약 15~30분 (OCR 검증 포함)
## 대상: 장량초 5학년 3반 (또는 모든 학급)

---

## 전체 흐름

```
인박스/  ← 선생님이 여기에 PDF + HWPX 템플릿 투입
  ├── 신체검사 결과표.pdf          (NEIS 출력물 또는 학교 기록지)
  └── 2026학년도 ... 가정통신문.hwpx  (메일머지 필드 설정된 양식)
         ↓
    [에이전트가 실행]
         1. OCR (GPT-5.5 Vision)
         2. 검증용 CSV 제공 → 선생님 확인
         3. 메일머지 생성
         4. 검증
         ↓
인박스/_agent_작업/가정통신문_출력물/
  ├── 5학년_3반_1번_공서희.hwpx
  ├── 5학년_3반_2번_김민아.hwpx
  └── ...
```

## Step 1 — PDF 데이터 추출 (OCR)

PDF는 **손글씨 스캔 문서**입니다. 한글 + 숫자 손글씨는 EasyOCR/Tesseract로 인식률이 매우 낮습니다.

**반드시 GPT-5.5 Vision에 위임** — `delegate_task`로 vision-capable subagent 사용:

```
delegate_task(
    goal="이 PDF 이미지에서 학생 신체검사 데이터를 추출해줘",
    context="PDF 경로, 출력 형식(CSV), 필드 목록",
    toolsets=["vision"]
)
```

출력: 학생명단 CSV (이름/성별/키/몸무게/시력좌/시력우/안경착용)

### OCR 팁
- PDF 페이지를 이미지로 변환 후 Vision에 전달
- 24명 기준 보통 3~8건 오인식
- 번호 순서대로 정렬되어 있으니 순번 기준으로 검증
- 안경착용 컬럼은 체크박스 — "예" 또는 공백

## Step 2 — 사용자 검증

OCR 결과를 UTF-8 BOM CSV로 제공:
```
_agent_작업/메일머지_워크플로우/ocr_결과_20260521.csv
```

사용자가 보내는 수정 패턴:
```
"단예준 137.7"        → 단예준의 키를 137.7로 수정
"유예준 시력 0.1 1.2"  → 유예준의 시력을 좌0.1 우1.2로 수정
```

### 반영 방법

```python
# update_csv.py 스크립트 사용
python update_csv.py ocr_결과.csv "단예준 137.7"
python update_csv.py ocr_결과.csv "유예준 시력 0.1 1.2"
```

또는 사용자가 직접 엑셀에서 CSV 열어서 수정.

## Step 3 — HWPX 메일머지 생성
### ❌ 절대 금지
- XML을 문자열로 replace/re.sub/f-string 조작 → **파일 깨짐**
- linesegarray 미삭제 → **한글에서 글자 겹침**
- 원본 덮어쓰기

### ✅ 올바른 방법
- `lxml.etree`로 파싱 → 수정 → 직렬화
- 수정한 모든 `<hp:p>`에서 `<linesegarray>` 삭제
- 출력은 `_agent_작업/`에 별도 폴더

```bash
python mailmerge.py "인박스/템플릿.hwpx" "ocr_결과.csv" "인박스/_agent_작업/가정통신문_출력물"
```

## Step 4 — 검증

mailmerge.py가 자동 수행:
1. ZIP 무결성 (`zf.testzip()`)
2. XML 내 `{{...}}` 잔여 플레이스홀더 검사
3. 값이 제대로 치환되었는지 샘플 2~3개 열어보기

---

## 빠른 실행 (에이전트 명령)

```
인박스에 PDF랑 템플릿 넣었어. 신체검사 메일머지 돌려줘.
```

→ 에이전트가 자동으로 Step 1~4 수행.

---

## 재사용 방법

### 내년에 같은 학급
1. 템플릿 재사용 (학년/반만 다름)
2. 새 PDF만 OCR
3. CSV의 학년/반 컬럼만 수정

### 다른 학급
1. 같은 템플릿 사용 가능 (메일머지 필드 동일)
2. 새 PDF OCR

---

## 참고 스킬
- `hwpx-mailmerge` — 메일머지 전용 스킬
- `hwpx-analyze` — HWPX 필드 분석
- `OneDrive/학습자료/사회/공책정리/hwpx-master.skill` — HWPX 마스터 가이드
