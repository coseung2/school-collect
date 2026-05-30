-- 카드 지출 대장 + 출장여비 신청 스키마
-- (기존 테이블과 충돌 없음, IF NOT EXISTS 사용)

-- 1. 법인카드 정보
CREATE TABLE IF NOT EXISTS card_expense_cards (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  card_name TEXT NOT NULL,
  card_number TEXT NOT NULL UNIQUE,
  card_holder TEXT DEFAULT '',
  issuing_bank TEXT DEFAULT '',
  is_active BOOLEAN NOT NULL DEFAULT true,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 2. 카드 수령 기록 (대여/반납)
CREATE TABLE IF NOT EXISTS card_assignments (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  card_id UUID NOT NULL REFERENCES card_expense_cards(id) ON DELETE CASCADE,
  teacher_name TEXT NOT NULL,
  taken_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  returned_at TIMESTAMPTZ,
  note TEXT DEFAULT '',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_ca_card ON card_assignments(card_id);
CREATE INDEX IF NOT EXISTS idx_ca_active ON card_assignments(returned_at) WHERE returned_at IS NULL;

-- 3. 지출 내역 (문자 웹훅 or 수동 등록)
CREATE TABLE IF NOT EXISTS card_expenses (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  card_id UUID NOT NULL REFERENCES card_expense_cards(id) ON DELETE CASCADE,
  assignment_id UUID REFERENCES card_assignments(id),
  teacher_name TEXT DEFAULT '',
  amount INTEGER NOT NULL,
  merchant TEXT NOT NULL,
  merchant_category TEXT DEFAULT '',
  expense_date TIMESTAMPTZ NOT NULL DEFAULT now(),
  receipt_image_url TEXT DEFAULT '',
  memo TEXT DEFAULT '',
  source TEXT NOT NULL DEFAULT 'manual'
    CHECK (source IN ('manual', 'sms_webhook', 'card_statement')),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_ce_card ON card_expenses(card_id);
CREATE INDEX IF NOT EXISTS idx_ce_date ON card_expenses(expense_date DESC);
CREATE INDEX IF NOT EXISTS idx_ce_teacher ON card_expenses(teacher_name);

-- 4. 지역별 거리 기준
CREATE TABLE IF NOT EXISTS trip_distance_std (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  region TEXT NOT NULL UNIQUE,
  base_distance INTEGER NOT NULL,
  round_trip BOOLEAN NOT NULL DEFAULT true,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 5. 출장여비 신청서
CREATE TABLE IF NOT EXISTS trip_expense_requests (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  teacher_name TEXT NOT NULL,
  department TEXT DEFAULT '',
  destination TEXT NOT NULL,
  purpose TEXT NOT NULL,
  transport TEXT NOT NULL DEFAULT 'car'
    CHECK (transport IN ('car', 'bus', 'train', 'etc')),
  trip_date DATE NOT NULL,
  distance INTEGER NOT NULL DEFAULT 0,
  fuel_unit_price INTEGER DEFAULT 0,
  fuel_efficiency REAL DEFAULT 10.5,
  fuel_cost INTEGER DEFAULT 0,
  toll_cost INTEGER DEFAULT 0,
  parking_cost INTEGER DEFAULT 0,
  other_cost INTEGER DEFAULT 0,
  total_cost INTEGER DEFAULT 0,
  receipt_image_url TEXT DEFAULT '',
  note TEXT DEFAULT '',
  status TEXT NOT NULL DEFAULT 'draft'
    CHECK (status IN ('draft', 'submitted', 'approved', 'rejected')),
  submitted_at TIMESTAMPTZ,
  reviewed_by TEXT DEFAULT '',
  review_note TEXT DEFAULT '',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_ter_date ON trip_expense_requests(trip_date DESC);
CREATE INDEX IF NOT EXISTS idx_ter_teacher ON trip_expense_requests(teacher_name);
CREATE INDEX IF NOT EXISTS idx_ter_status ON trip_expense_requests(status);

-- 시드 데이터: 장량초 기준 출장 거리 기준
INSERT INTO trip_distance_std (region, base_distance, round_trip) VALUES
  ('포항시내', 15, true),
  ('경주', 35, true),
  ('영천', 60, true),
  ('대구', 130, true),
  ('부산', 130, true),
  ('울산', 100, true),
  ('창원', 140, true),
  ('안동', 120, true),
  ('구미', 140, true),
  ('서울', 370, true),
  ('대전', 240, true),
  ('광주', 280, true),
  ('전주', 300, true),
  ('강릉', 300, true)
ON CONFLICT (region) DO NOTHING;
