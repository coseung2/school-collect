-- 월중 교육 계획처럼 '학년/반'이 아닌 교직원/부서 단위 수합을 안정적으로 처리하기 위한 제안 SQL
-- Supabase SQL Editor에서 실행하기 전, 기존 제출 upsert 로직도 함께 수정해야 함.

ALTER TABLE collect_submissions
  ADD COLUMN IF NOT EXISTS submitter_key TEXT,
  ADD COLUMN IF NOT EXISTS department TEXT,
  ADD COLUMN IF NOT EXISTS role_label TEXT;

-- 학급 제출은 기존 UNIQUE(run_id, grade, class_num)를 유지.
-- 교직원/부서 제출은 submitter_key로 중복 방지.
CREATE UNIQUE INDEX IF NOT EXISTS uniq_collect_submissions_run_submitter_key
ON collect_submissions(run_id, submitter_key)
WHERE submitter_key IS NOT NULL;
