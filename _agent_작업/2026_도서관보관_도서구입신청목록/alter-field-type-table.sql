-- 반복행 테이블 입력을 위한 field_type 확장 SQL
-- Supabase Dashboard → SQL Editor에서 실행
-- 실행 전: 앱 코드에 table 렌더링 분기가 들어간 뒤 적용하는 것을 권장

ALTER TABLE form_fields
DROP CONSTRAINT IF EXISTS form_fields_field_type_check;

ALTER TABLE form_fields
ADD CONSTRAINT form_fields_field_type_check
CHECK (field_type IN ('text','number','date','select','textarea','table'));
