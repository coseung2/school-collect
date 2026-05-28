# 학교 업무수합 플랫폼 (School Collect)

교내 각종 수합 업무를 디지털화하는 플랫폼입니다.

## 기능

- **수합 등록** — 담당자가 양식과 기한만 지정
- **바로 입력** — 선생님들이 대시보드에서 폼 작성
- **자동 취합** — 반 → 학년 → 학교 전체 자동 집계
- **재사용** — 작년 양식을 템플릿으로 저장했다가 내년에 재사용
- **CSV 다운로드** — 취합 결과를 엑셀/CSV로 내보내기

## 기술 스택

| 항목 | 선택 |
|------|------|
| 프레임워크 | Next.js 16 (App Router) |
| DB | Supabase (PostgreSQL) |
| API | Supabase JS SDK |
| 스타일 | Tailwind CSS |
| 배포 | Vercel |

## 시작하기

```bash
# 의존성 설치
npm install

# .env 설정
cp .env.example .env
# → .env에 Supabase URL + 키가 이미 있음

# Supabase SQL Editor에서 테이블 생성
# supabase-schema.sql 파일 내용 복사 → Supabase Dashboard → SQL Editor → 실행

# 개발 서버
npm run dev
```

## DB 스키마

```
form_templates    → 양식 템플릿 (연도별 재사용 가능)
  └─ form_fields  → 템플릿 필드들
collect_runs      → 특정 연도의 수합 실행
  └─ collect_submissions → 각 반/학년의 제출
       └─ collect_answers → 각 필드별 답변
```

## 테이블 생성 방법

`supabase-schema.sql`을 열어서 전체 내용 복사 →
[Supabase Dashboard](https://supabase.com/dashboard/project/jwigtluovdmwjkcqiofm) →
**SQL Editor** → 붙여넣기 → **Run** 한 번이면 끝.

## 구조

```
src/
├── app/
│   ├── page.tsx                  # 대시보드
│   ├── admin/
│   │   ├── page.tsx              # 수합 관리
│   │   └── new/page.tsx          # 새 수합 등록
│   ├── collect/
│   │   └── [id]/
│   │       ├── page.tsx          # 입력 폼
│   │       └── results/page.tsx  # 취합 현황 + CSV
│   └── api/
│       ├── templates/route.ts    # 템플릿 CRUD
│       ├── runs/route.ts         # 수합 실행 CRUD
│       └── submissions/route.ts  # 제출 CRUD
└── lib/
    ├── supabase.ts               # Supabase 클라이언트
    └── types.ts                  # 공통 타입
```
