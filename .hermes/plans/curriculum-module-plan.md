# School-Collect — 교육과정 작성 모듈 (v2, GPT-5.5 검수 반영)

## 검수 결과 (GPT-5.5, 2026-06-03)
❌ 보완필요 → 전면 반영

## 반영된 주요 변경사항
1. **라우트 통일**: 페이지 → `/admin/curriculum/*`, API → `/api/curriculum/*`
2. **curriculum_years + curriculum_classes** 추가 (학년도 작성 상태 관리, 학년/반/담임 배정)
3. **Bulk upsert** 엔드포인트 (시간표 그리드용)
4. **검증 엔드포인트** (시수 충족, 누락 차시)
5. school_years 유지하되 curriculum_years가 작성 워크플로우 루트

## DB 스키마 (추가/변경분)

### 1. curriculum_years — 교육과정 작성 마스터
```sql
CREATE TABLE curriculum_years (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  school_id UUID NOT NULL REFERENCES sc_schools(id),
  year INTEGER NOT NULL,
  status TEXT NOT NULL DEFAULT 'draft'
    CHECK (status IN ('draft','editing','validated','confirmed','archived')),
  is_active BOOLEAN NOT NULL DEFAULT true,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(school_id, year)
);
```

### 2. curriculum_classes — 학년/반 설정
```sql
CREATE TABLE curriculum_classes (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  curriculum_year_id UUID NOT NULL REFERENCES curriculum_years(id) ON DELETE CASCADE,
  grade INTEGER NOT NULL CHECK (grade BETWEEN 1 AND 6),
  class_no INTEGER NOT NULL,
  homeroom_teacher_id UUID REFERENCES sc_teachers(id),
  UNIQUE(curriculum_year_id, grade, class_no)
);
```

### 3. 기존 테이블 보강
- `school_years`: school_id 컬럼 추가
- `curriculum_hours`: term1_hours, term2_hours, required_hours 추가
- `calendar_events`: is_school_day, affects_timetable, affected_periods, grade_scope 추가
- `cross_topic_logs`: hours → NUMERIC(3,1) (0.5차시 지원)
- `elective_subjects`: subject_id 컬럼 추가 (subjects와 연결)

## API Routes (/api/curriculum/*)

| Route | Method | 설명 |
|---|---|---|
| `/api/curriculum/years` | GET, POST, PATCH | curriculum_years CRUD |
| `/api/curriculum/classes` | GET, POST | curriculum_classes |
| `/api/curriculum/subjects` | GET, POST | 교과 (기존 유지) |
| `/api/curriculum/hours` | GET, POST, PATCH | 시수편제 |
| `/api/curriculum/bulk/hours` | POST | 시수편제 bulk upsert |
| `/api/curriculum/calendar-events` | GET, POST, DELETE | 학사일정 |
| `/api/curriculum/timetables` | GET, POST, PATCH | 시간표 (source_type: base/semester) |
| `/api/curriculum/timetables/copy` | POST | 기초→학기 복사 |
| `/api/curriculum/timetables/bulk` | POST | 시간표 bulk upsert |
| `/api/curriculum/lessons` | GET, POST, PATCH, DELETE | 진도표 |
| `/api/curriculum/lessons/bulk` | POST | 진도표 bulk upsert |
| `/api/curriculum/projects` | GET, POST, PATCH, DELETE | 주제통합 |
| `/api/curriculum/cross-topics` | GET, POST, PATCH | 범교과주제 |
| `/api/curriculum/cross-topic-logs` | GET, POST | 범교과 기록 |
| `/api/curriculum/electives` | GET, POST | 학교자율과목 |
| `/api/curriculum/validate` | GET | 교육과정 검증 |

## 페이지 라우트 (/admin/curriculum/*)

| Path | 설명 |
|---|---|
| `/admin/curriculum` | 작성 대시보드 (학년도 목록 + 진행률) |
| `/admin/curriculum/years/new` | 새 학년도 생성 |
| `/admin/curriculum/years/[id]` | 학년도 상세 (단계별 작성) |
| `/admin/curriculum/classes/[yearId]` | 학년/반 설정 + 담임 배정 |
| `/admin/curriculum/hours/[yearId]` | 시수편제 |
| `/admin/curriculum/calendar/[yearId]` | 학사일정 |
| `/admin/curriculum/timetable/base/[yearId]` | 기초시간표 |
| `/admin/curriculum/timetable/semester/[yearId]` | 학기별 시간표 |
| `/admin/curriculum/lessons/[yearId]` | 교과 진도표 |
| `/admin/curriculum/projects/[yearId]` | 주제통합 프로젝트 |
| `/admin/curriculum/cross-topics/[yearId]` | 범교과주제 |
| `/admin/curriculum/electives/[yearId]` | 학교자율과목 |
| `/admin/curriculum/validate/[yearId]` | 검증 |

## 구현 순서

1. DB 스키마 추가 SQL + types.ts 갱신
2. API routes (/api/curriculum/*) — years, classes, timetables, lessons, etc.
3. 페이지 (/admin/curriculum/*) — 순서대로
4. 구 route 파일 정리
5. DB 마이그레이션 실행
