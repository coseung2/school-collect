-- ============================================================
-- School-Collect: Full Schema Migration (v2 with GPT-5.5 review)
-- Run ONCE in Supabase Dashboard SQL Editor or via supabase CLI
-- ============================================================

-- ============================================================
-- DROP EXISTING CURRICULUM TABLES (safe cleanup if re-running)
-- ============================================================
DROP TABLE IF EXISTS cross_topic_logs CASCADE;
DROP TABLE IF EXISTS cross_topics CASCADE;
DROP TABLE IF EXISTS curriculum_project_subjects CASCADE;
DROP TABLE IF EXISTS curriculum_projects CASCADE;
DROP TABLE IF EXISTS lessons CASCADE;
DROP TABLE IF EXISTS timetable_semester CASCADE;
DROP TABLE IF EXISTS timetable_base CASCADE;
DROP TABLE IF EXISTS teacher_subject_assignments CASCADE;
DROP TABLE IF EXISTS curriculum_classes CASCADE;
DROP TABLE IF EXISTS curriculum_hours CASCADE;
DROP TABLE IF EXISTS calendar_events CASCADE;
DROP TABLE IF EXISTS school_years CASCADE;
DROP TABLE IF EXISTS elective_timetable CASCADE;
DROP TABLE IF EXISTS elective_subjects CASCADE;
DROP TABLE IF EXISTS subjects CASCADE;
DROP TABLE IF EXISTS curriculum_years CASCADE;
DROP TABLE IF EXISTS form_fields CASCADE;
DROP TABLE IF EXISTS form_templates CASCADE;
DROP TABLE IF EXISTS collect_submissions CASCADE;
DROP TABLE IF EXISTS collect_answers CASCADE;
DROP TABLE IF EXISTS collect_runs CASCADE;

-- ============================================================
-- 1. 양식 템플릿
-- ============================================================
CREATE TABLE form_templates (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  title TEXT NOT NULL,
  description TEXT,
  year INTEGER NOT NULL DEFAULT 2026,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE form_fields (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  template_id UUID NOT NULL REFERENCES form_templates(id) ON DELETE CASCADE,
  label TEXT NOT NULL,
  field_type TEXT NOT NULL DEFAULT 'text'
    CHECK (field_type IN ('text','number','date','select','textarea')),
  required BOOLEAN NOT NULL DEFAULT false,
  options JSONB,
  "order" INTEGER NOT NULL DEFAULT 0,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_form_fields_template ON form_fields(template_id);

-- ============================================================
-- 2. 수합 실행
-- ============================================================
CREATE TABLE collect_runs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  template_id UUID NOT NULL REFERENCES form_templates(id) ON DELETE CASCADE,
  title TEXT,
  year INTEGER NOT NULL DEFAULT 2026,
  deadline TIMESTAMPTZ NOT NULL,
  status TEXT NOT NULL DEFAULT 'open'
    CHECK (status IN ('open','closed','archived')),
  target_type TEXT NOT NULL DEFAULT 'all'
    CHECK (target_type IN ('all','grade','class','custom')),
  target_value TEXT,
  description TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE collect_submissions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  run_id UUID NOT NULL REFERENCES collect_runs(id) ON DELETE CASCADE,
  grade INTEGER,
  class_num INTEGER,
  submitter TEXT,
  submitted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE collect_answers (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  submission_id UUID NOT NULL REFERENCES collect_submissions(id) ON DELETE CASCADE,
  field_id UUID NOT NULL REFERENCES form_fields(id) ON DELETE CASCADE,
  value TEXT NOT NULL DEFAULT '',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ============================================================
-- 3. 교과 (기준 데이터)
-- ============================================================
CREATE TABLE subjects (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  code TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  category TEXT NOT NULL DEFAULT 'general'
    CHECK (category IN ('general', 'creative', 'discretionary', 'elective')),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ============================================================
-- 4. 학년도/학기
-- ============================================================
CREATE TABLE school_years (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  school_id UUID REFERENCES sc_schools(id),
  year INTEGER NOT NULL,
  term INTEGER NOT NULL CHECK (term IN (1, 2)),
  active BOOLEAN NOT NULL DEFAULT true,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(year, term)
);

-- ============================================================
-- 5. 교육과정 마스터
-- ============================================================
CREATE TABLE curriculum_years (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  school_id UUID NOT NULL REFERENCES sc_schools(id),
  year INTEGER NOT NULL,
  status TEXT NOT NULL DEFAULT 'draft'
    CHECK (status IN ('draft', 'editing', 'validated', 'confirmed', 'archived')),
  is_active BOOLEAN NOT NULL DEFAULT true,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(school_id, year)
);

-- ============================================================
-- 6. 학년/반 설정
-- ============================================================
CREATE TABLE curriculum_classes (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  curriculum_year_id UUID NOT NULL REFERENCES curriculum_years(id) ON DELETE CASCADE,
  grade INTEGER NOT NULL CHECK (grade BETWEEN 1 AND 6),
  class_no INTEGER NOT NULL,
  homeroom_teacher_id UUID REFERENCES sc_teachers(id),
  student_count INTEGER DEFAULT 0,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(curriculum_year_id, grade, class_no)
);

-- ============================================================
-- 7. 교과전담 배정
-- ============================================================
CREATE TABLE teacher_subject_assignments (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  curriculum_year_id UUID NOT NULL REFERENCES curriculum_years(id) ON DELETE CASCADE,
  teacher_id UUID NOT NULL REFERENCES sc_teachers(id) ON DELETE CASCADE,
  subject_id UUID NOT NULL REFERENCES subjects(id),
  grade INTEGER CHECK (grade BETWEEN 1 AND 6),
  class_no INTEGER,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(curriculum_year_id, teacher_id, subject_id, grade, class_no)
);

-- ============================================================
-- 8. 시수편제
-- ============================================================
CREATE TABLE curriculum_hours (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  year_id UUID NOT NULL REFERENCES school_years(id) ON DELETE CASCADE,
  grade INTEGER NOT NULL CHECK (grade BETWEEN 1 AND 6),
  subject_id UUID NOT NULL REFERENCES subjects(id),
  total_hours INTEGER NOT NULL,
  term1_hours INTEGER,
  term2_hours INTEGER,
  required_hours INTEGER,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(year_id, grade, subject_id)
);

-- ============================================================
-- 9. 학사일정
-- ============================================================
CREATE TABLE calendar_events (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  year_id UUID NOT NULL REFERENCES school_years(id) ON DELETE CASCADE,
  date DATE NOT NULL,
  type TEXT NOT NULL CHECK (type IN ('holiday', 'event', 'break', 'exam', 'etc')),
  title TEXT NOT NULL,
  description TEXT,
  is_school_day BOOLEAN DEFAULT true,
  affects_timetable BOOLEAN DEFAULT false,
  affected_periods INTEGER[],
  grade_scope INTEGER[],
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_calendar_events_date ON calendar_events(year_id, date);

-- ============================================================
-- 10. 기초시간표
-- ============================================================
CREATE TABLE timetable_base (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  year_id UUID NOT NULL REFERENCES school_years(id) ON DELETE CASCADE,
  grade INTEGER NOT NULL CHECK (grade BETWEEN 1 AND 6),
  class_no INTEGER NOT NULL,
  weekday INTEGER NOT NULL CHECK (weekday BETWEEN 1 AND 5),
  period INTEGER NOT NULL CHECK (period BETWEEN 1 AND 7),
  subject_id UUID REFERENCES subjects(id),
  teacher_id UUID,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(year_id, grade, class_no, weekday, period)
);
CREATE INDEX IF NOT EXISTS idx_timetable_base_grid ON timetable_base(year_id, grade, class_no);

-- ============================================================
-- 11. 학기별 시간표
-- ============================================================
CREATE TABLE timetable_semester (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  year_id UUID NOT NULL REFERENCES school_years(id) ON DELETE CASCADE,
  term INTEGER NOT NULL CHECK (term IN (1, 2)),
  grade INTEGER NOT NULL CHECK (grade BETWEEN 1 AND 6),
  class_no INTEGER NOT NULL,
  weekday INTEGER NOT NULL CHECK (weekday BETWEEN 1 AND 5),
  period INTEGER NOT NULL CHECK (period BETWEEN 1 AND 7),
  subject_id UUID REFERENCES subjects(id),
  teacher_id UUID,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(year_id, term, grade, class_no, weekday, period)
);
CREATE INDEX IF NOT EXISTS idx_timetable_semester_grid ON timetable_semester(year_id, term, grade, class_no);

-- ============================================================
-- 12. 교과 진도표
-- ============================================================
CREATE TABLE lessons (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  year_id UUID NOT NULL REFERENCES school_years(id) ON DELETE CASCADE,
  term INTEGER NOT NULL CHECK (term IN (1, 2)),
  grade INTEGER NOT NULL CHECK (grade BETWEEN 1 AND 6),
  class_no INTEGER NOT NULL,
  subject_id UUID NOT NULL REFERENCES subjects(id),
  unit TEXT NOT NULL,
  lesson_no INTEGER NOT NULL,
  content TEXT NOT NULL,
  start_date DATE,
  end_date DATE,
  period INTEGER,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(year_id, term, grade, class_no, subject_id, lesson_no)
);
CREATE INDEX IF NOT EXISTS idx_lessons_subject ON lessons(year_id, term, grade, subject_id);

-- ============================================================
-- 13. 교과 주제통합 (프로젝트) - renamed to avoid conflict with existing projects table
-- ============================================================
CREATE TABLE curriculum_projects (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  year_id UUID NOT NULL REFERENCES school_years(id) ON DELETE CASCADE,
  term INTEGER NOT NULL CHECK (term IN (1, 2)),
  grade INTEGER NOT NULL CHECK (grade BETWEEN 1 AND 6),
  class_no INTEGER NOT NULL,
  title TEXT NOT NULL,
  description TEXT,
  start_date DATE,
  end_date DATE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(year_id, term, grade, class_no, title)
);

CREATE TABLE curriculum_project_subjects (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  project_id UUID NOT NULL REFERENCES curriculum_projects(id) ON DELETE CASCADE,
  subject_id UUID NOT NULL REFERENCES subjects(id),
  lesson_id UUID REFERENCES lessons(id),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(project_id, subject_id)
);

-- ============================================================
-- 14. 범교과주제
-- ============================================================
CREATE TABLE cross_topics (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  year_id UUID NOT NULL REFERENCES school_years(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  category TEXT NOT NULL,
  default_hours NUMERIC(3,1) NOT NULL DEFAULT 0,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(year_id, name)
);

CREATE TABLE cross_topic_logs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  year_id UUID NOT NULL REFERENCES school_years(id) ON DELETE CASCADE,
  lesson_id UUID NOT NULL REFERENCES lessons(id) ON DELETE CASCADE,
  topic_id UUID NOT NULL REFERENCES cross_topics(id),
  hours NUMERIC(3,1) NOT NULL DEFAULT 1,
  note TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(lesson_id, topic_id)
);
CREATE INDEX IF NOT EXISTS idx_cross_topic_logs_topic ON cross_topic_logs(topic_id);

-- ============================================================
-- 15. 학교자율과목
-- ============================================================
CREATE TABLE elective_subjects (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  year_id UUID NOT NULL REFERENCES school_years(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  grade INTEGER CHECK (grade BETWEEN 1 AND 6),
  total_hours INTEGER NOT NULL,
  subject_id UUID REFERENCES subjects(id),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(year_id, name, grade)
);

-- ============================================================
-- 16. 추가 인덱스
-- ============================================================
CREATE INDEX IF NOT EXISTS idx_curriculum_hours_grade ON curriculum_hours(year_id, grade);
CREATE INDEX IF NOT EXISTS idx_curriculum_classes_year ON curriculum_classes(curriculum_year_id);
CREATE INDEX IF NOT EXISTS idx_teacher_assignments_year ON teacher_subject_assignments(curriculum_year_id);

-- ============================================================
-- 17. Refresh schema cache
-- ============================================================
NOTIFY pgrst, 'reload schema';
