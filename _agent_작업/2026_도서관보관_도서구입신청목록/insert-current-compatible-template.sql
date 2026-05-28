-- 현재 school-collect 구조에서 바로 쓸 수 있는 임시 도서구입 양식 등록 SQL
-- 권장: 정식 구현 전 급하게 수합해야 할 때만 사용
-- 정식 구현은 recommended-table-template.json 기준의 반복행 테이블 UI가 좋습니다.

DO $$
DECLARE
  v_template_id uuid;
BEGIN
  SELECT id INTO v_template_id
  FROM form_templates
  WHERE title = '2026학년도 1차 도서관 보관 도서구입 신청 목록(장량초등학교) - 임시 붙여넣기형' AND year = 2026
  LIMIT 1;

  IF v_template_id IS NULL THEN
    INSERT INTO form_templates (title, description, year)
    VALUES ('2026학년도 1차 도서관 보관 도서구입 신청 목록(장량초등학교) - 임시 붙여넣기형', '도서구입 신청 목록을 한 번에 붙여넣어 제출하는 임시 양식입니다. 정식 구현은 반복행 테이블 입력 방식이 권장됩니다.', 2026)
    RETURNING id INTO v_template_id;
  ELSE
    UPDATE form_templates
    SET description = '도서구입 신청 목록을 한 번에 붙여넣어 제출하는 임시 양식입니다. 정식 구현은 반복행 테이블 입력 방식이 권장됩니다.', updated_at = now()
    WHERE id = v_template_id;
    DELETE FROM form_fields WHERE template_id = v_template_id;
  END IF;

  INSERT INTO form_fields (template_id, label, field_type, required, options, "order")
  VALUES
    (v_template_id, '도서구입목록_CSV붙여넣기', 'textarea', true, NULL::jsonb, 0),
    (v_template_id, '총권수', 'number', false, NULL::jsonb, 1),
    (v_template_id, '총금액', 'number', false, NULL::jsonb, 2),
    (v_template_id, '특이사항', 'textarea', false, NULL::jsonb, 3);
END $$;
