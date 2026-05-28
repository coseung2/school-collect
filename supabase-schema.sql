-- ============================================================
-- 학교 업무수합 플랫폼 (School Collect) — DB 스키마
-- Supabase Dashboard → SQL Editor 에서 실행
-- ============================================================

-- 1. 양식 템플릿 (연도별로 재사용 가능)
CREATE TABLE IF NOT EXISTS form_templates (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  title TEXT NOT NULL,
  description TEXT,
  year INTEGER NOT NULL DEFAULT 2026,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 2. 템플릿 필드
CREATE TABLE IF NOT EXISTS form_fields (
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

-- 3. 수합 실행 (특정 연도의 실제 수합)
CREATE TABLE IF NOT EXISTS collect_runs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  template_id UUID NOT NULL REFERENCES form_templates(id),
  year INTEGER NOT NULL DEFAULT 2026,
  deadline TIMESTAMPTZ NOT NULL,
  status TEXT NOT NULL DEFAULT 'open'
    CHECK (status IN ('open','closed','archived')),
  target_type TEXT NOT NULL DEFAULT 'all'
    CHECK (target_type IN ('all','grade','class','custom')),
  target_value JSONB,
  description TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_collect_runs_year ON collect_runs(year DESC);
CREATE INDEX IF NOT EXISTS idx_collect_runs_status ON collect_runs(status);

-- 4. 제출 (한 반/학년의 입력)
CREATE TABLE IF NOT EXISTS collect_submissions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  run_id UUID NOT NULL REFERENCES collect_runs(id) ON DELETE CASCADE,
  grade INTEGER,
  class_num INTEGER,
  submitter TEXT,
  submitter_id TEXT,
  submitted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(run_id, grade, class_num)
);
CREATE INDEX IF NOT EXISTS idx_collect_submissions_run ON collect_submissions(run_id);

-- 5. 필드별 답변
CREATE TABLE IF NOT EXISTS collect_answers (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  submission_id UUID NOT NULL REFERENCES collect_submissions(id) ON DELETE CASCADE,
  field_id UUID NOT NULL REFERENCES form_fields(id),
  value TEXT,
  UNIQUE(submission_id, field_id)
);
CREATE INDEX IF NOT EXISTS idx_collect_answers_submission ON collect_answers(submission_id);

-- ✅ 완료
-- ============================================================
-- 월중계획 전용 테이블
-- ============================================================

-- 월중계획 (한 달 단위 수합)
CREATE TABLE IF NOT EXISTS monthly_plans (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  year INTEGER NOT NULL DEFAULT 2026,
  month INTEGER NOT NULL CHECK (month BETWEEN 1 AND 12),
  title TEXT NOT NULL,
  description TEXT,
  deadline TIMESTAMPTZ,
  status TEXT NOT NULL DEFAULT 'open'
    CHECK (status IN ('open','collecting','reviewing','published','archived')),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_monthly_plans_date ON monthly_plans(year DESC, month DESC);

-- 개별 일정 항목 (한 교사가 입력한 여러 일정)
CREATE TABLE IF NOT EXISTS monthly_plan_entries (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  plan_id UUID NOT NULL REFERENCES monthly_plans(id) ON DELETE CASCADE,
  plan_date DATE NOT NULL,
  content TEXT NOT NULL,
  target TEXT DEFAULT '',
  location TEXT DEFAULT '',
  person_in_charge TEXT DEFAULT '',
  notes TEXT DEFAULT '',
  submitter TEXT NOT NULL DEFAULT '',
  week_number INTEGER GENERATED ALWAYS AS (EXTRACT(WEEK FROM plan_date)) STORED,
  day_of_week INTEGER GENERATED ALWAYS AS (EXTRACT(DOW FROM plan_date)) STORED,
  status TEXT NOT NULL DEFAULT 'draft'
    CHECK (status IN ('draft','submitted','confirmed','rejected')),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_mpe_plan ON monthly_plan_entries(plan_id);
CREATE INDEX IF NOT EXISTS idx_mpe_date ON monthly_plan_entries(plan_date);
CREATE INDEX IF NOT EXISTS idx_mpe_status ON monthly_plan_entries(status);

-- ✅ 월중계획 스키마 완료

-- ============================================================
-- 메일머지 (가정통신문) 스키마
-- ============================================================

-- 1. 메일머지 템플릿 (HWPX 양식 정보)
CREATE TABLE IF NOT EXISTS mailmerge_templates (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  title TEXT NOT NULL,
  description TEXT,
  document_type TEXT NOT NULL DEFAULT 'general'
    CHECK (document_type IN ('general','health_check','dental','urine','vision','grade_report','consent','notice','etc')),
  year INTEGER NOT NULL DEFAULT 2026,
  fields JSONB NOT NULL DEFAULT '[]'::jsonb,
  template_file_path TEXT,
  is_active BOOLEAN NOT NULL DEFAULT true,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_mmt_year ON mailmerge_templates(year DESC);
CREATE INDEX IF NOT EXISTS idx_mmt_type ON mailmerge_templates(document_type);
CREATE INDEX IF NOT EXISTS idx_mmt_active ON mailmerge_templates(is_active);

-- 2. 메일머지 실행 기록
CREATE TABLE IF NOT EXISTS mailmerge_runs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  template_id UUID NOT NULL REFERENCES mailmerge_templates(id) ON DELETE CASCADE,
  title TEXT NOT NULL,
  grade INTEGER,
  class_num INTEGER,
  student_count INTEGER DEFAULT 0,
  data_file_path TEXT,
  status TEXT NOT NULL DEFAULT 'draft'
    CHECK (status IN ('draft','data_ready','processing','completed','failed')),
  corrections TEXT,
  output_path TEXT,
  error_message TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_mmr_template ON mailmerge_runs(template_id);
CREATE INDEX IF NOT EXISTS idx_mmr_status ON mailmerge_runs(status);
CREATE INDEX IF NOT EXISTS idx_mmr_created ON mailmerge_runs(created_at DESC);

-- ✅ 메일머지 스키마 완료

-- ============================================================
-- 정보화기기 구입요청 스키마
-- ============================================================

-- 1. 구입 물품 (관리자가 등록)
CREATE TABLE IF NOT EXISTS purchase_items (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL,
  spec TEXT DEFAULT '',
  unit_price INTEGER NOT NULL DEFAULT 0,
  unit TEXT NOT NULL DEFAULT 'EA',
  link TEXT DEFAULT '',
  description TEXT DEFAULT '',
  category TEXT DEFAULT '기타',
  is_active BOOLEAN NOT NULL DEFAULT true,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_pi_active ON purchase_items(is_active);

-- 2. 신청 내역 (교사가 클릭으로 신청)
CREATE TABLE IF NOT EXISTS purchase_requests (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  item_id UUID NOT NULL REFERENCES purchase_items(id) ON DELETE CASCADE,
  grade INTEGER NOT NULL,
  class_num INTEGER NOT NULL,
  quantity INTEGER NOT NULL DEFAULT 1,
  submitter TEXT DEFAULT '',
  notes TEXT DEFAULT '',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(item_id, grade, class_num)
);
CREATE INDEX IF NOT EXISTS idx_pr_item ON purchase_requests(item_id);
CREATE INDEX IF NOT EXISTS idx_pr_grade_class ON purchase_requests(grade, class_num);

-- ✅ 구입요청 스키마 완료

-- ============================================================
-- 개인정보파일 연간 정비 시스템 — privacy_* 스키마
-- (오우알파 코드 리뷰 후 추가: 2026-05-28)
-- ============================================================

-- 1. 표준 개인정보파일 목록 (교육지원청 표준 13개)
CREATE TABLE IF NOT EXISTS privacy_file_standards (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  business_area TEXT NOT NULL,           -- 업무분야 (교무, 보건, 행정실…)
  file_name TEXT NOT NULL,               -- 파일명
  retention_period TEXT NOT NULL,        -- 보유기간 (준영구, 5년, 10년…)
  department TEXT NOT NULL,              -- 담당부서 (교무부, 행정실…)
  guide TEXT,                            -- 정보주체 수 산정 가이드
  is_active BOOLEAN NOT NULL DEFAULT true,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_privacy_standards_dept ON privacy_file_standards(department);
CREATE INDEX IF NOT EXISTS idx_privacy_standards_active ON privacy_file_standards(is_active);

-- 2. 연간 정비 사이클 (매년 5월)
CREATE TABLE IF NOT EXISTS privacy_maintenance_cycles (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  year INTEGER NOT NULL UNIQUE,
  start_date DATE NOT NULL,
  end_date DATE NOT NULL,
  status TEXT NOT NULL DEFAULT 'draft'
    CHECK (status IN ('draft', 'active', 'completed')),
  description TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_privacy_cycles_year ON privacy_maintenance_cycles(year DESC);
CREATE INDEX IF NOT EXISTS idx_privacy_cycles_status ON privacy_maintenance_cycles(status);

-- 3. 부서 관리
CREATE TABLE IF NOT EXISTS privacy_departments (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL UNIQUE,
  display_order INTEGER NOT NULL DEFAULT 0,
  is_active BOOLEAN NOT NULL DEFAULT true,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_privacy_depts_active ON privacy_departments(is_active);

-- 4. 부서별 담당자 (연도 무관, 최신 1명 active)
CREATE TABLE IF NOT EXISTS privacy_department_managers (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  department_id UUID NOT NULL REFERENCES privacy_departments(id),
  name TEXT NOT NULL,
  position TEXT,
  phone TEXT,
  email TEXT,
  is_active BOOLEAN NOT NULL DEFAULT true,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_privacy_managers_dept ON privacy_department_managers(department_id);
CREATE UNIQUE INDEX IF NOT EXISTS privacy_department_managers_department_id_is_active_key
  ON privacy_department_managers(department_id, is_active);

-- 5. 정비 입력 데이터 (사이클 × 부서 × 파일)
CREATE TABLE IF NOT EXISTS privacy_maintenance_entries (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  cycle_id UUID NOT NULL REFERENCES privacy_maintenance_cycles(id),
  department_id UUID NOT NULL REFERENCES privacy_departments(id),
  standard_id UUID NOT NULL REFERENCES privacy_file_standards(id),
  data_subject_count INTEGER,
  has_anomaly BOOLEAN DEFAULT false,
  anomaly_description TEXT,
  manager_name TEXT,
  manager_position TEXT,
  notes TEXT,
  is_submitted BOOLEAN NOT NULL DEFAULT false,
  submitted_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(cycle_id, department_id, standard_id)
);
CREATE INDEX IF NOT EXISTS idx_privacy_entries_cycle ON privacy_maintenance_entries(cycle_id);
CREATE INDEX IF NOT EXISTS idx_privacy_entries_dept ON privacy_maintenance_entries(department_id);
CREATE INDEX IF NOT EXISTS idx_privacy_entries_submitted ON privacy_maintenance_entries(is_submitted);

-- 6. 연도별 히스토리 (정비 완료 시 스냅샷)
CREATE TABLE IF NOT EXISTS privacy_maintenance_history (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  cycle_id UUID NOT NULL REFERENCES privacy_maintenance_cycles(id),
  year INTEGER NOT NULL,
  export_data JSONB NOT NULL,
  submitted_by TEXT,
  submitted_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_privacy_history_year ON privacy_maintenance_history(year DESC);

-- ✅ 전체 스키마 완료

-- ============================================================
-- 권한/인증 시스템 — sc_* 스키마
-- (학교별 교사 계정 + 업무 배정)
-- ============================================================

-- 1. school-collect 전용 학교 정보
CREATE TABLE IF NOT EXISTS sc_schools (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL UNIQUE,
  edu_office_code TEXT,
  school_code TEXT,
  is_active BOOLEAN NOT NULL DEFAULT true,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 2. 교사 프로필 (Supabase Auth와 1:1)
CREATE TABLE IF NOT EXISTS sc_teachers (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  auth_user_id UUID UNIQUE NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
  school_id UUID NOT NULL REFERENCES sc_schools(id),
  name TEXT NOT NULL,
  email TEXT NOT NULL UNIQUE,
  phone TEXT,
  role TEXT NOT NULL DEFAULT 'teacher'
    CHECK (role IN ('admin', 'department_head', 'teacher')),
  is_active BOOLEAN NOT NULL DEFAULT true,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_sc_teachers_school ON sc_teachers(school_id);
CREATE INDEX IF NOT EXISTS idx_sc_teachers_role ON sc_teachers(role);

-- 3. 교사-부서 업무 배정 (1명 여러 부서)
CREATE TABLE IF NOT EXISTS sc_teacher_depts (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  teacher_id UUID NOT NULL REFERENCES sc_teachers(id) ON DELETE CASCADE,
  department_name TEXT NOT NULL,
  is_primary BOOLEAN NOT NULL DEFAULT false,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(teacher_id, department_name)
);
CREATE INDEX IF NOT EXISTS idx_sc_teacher_depts_teacher ON sc_teacher_depts(teacher_id);
