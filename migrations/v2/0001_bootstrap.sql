CREATE TABLE IF NOT EXISTS app_meta (
    singleton_id SMALLINT PRIMARY KEY DEFAULT 1 CHECK (singleton_id = 1),
    schema_version TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO app_meta (singleton_id, schema_version)
VALUES (1, 'v2-bootstrap')
ON CONFLICT (singleton_id) DO NOTHING;
