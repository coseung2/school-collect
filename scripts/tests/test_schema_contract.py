from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
MIGRATION = ROOT / "migrations" / "v2" / "0002_identity_tenancy_collect.sql"


class SchemaContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.sql = MIGRATION.read_text(encoding="utf-8")

    def test_identity_and_collect_tables_are_present(self):
        for table in (
            "tenants",
            "users",
            "memberships",
            "collects",
            "collect_items",
            "collect_assignments",
            "collect_submissions",
            "audit_events",
            "outbox_events",
        ):
            self.assertIn(f"CREATE TABLE IF NOT EXISTS school_collect.{table}", self.sql)

    def test_v2_objects_live_in_a_dedicated_schema(self):
        self.assertIn("CREATE SCHEMA IF NOT EXISTS school_collect", self.sql)
        self.assertNotIn("CREATE TABLE IF NOT EXISTS app_meta", self.sql)

    def test_tenant_and_version_invariants_are_present(self):
        self.assertIn("UNIQUE (issuer, subject)", self.sql)
        self.assertIn(
            "tenant_id UUID NOT NULL REFERENCES school_collect.tenants(id)", self.sql
        )
        self.assertIn("version BIGINT NOT NULL DEFAULT 0 CHECK (version >= 0)", self.sql)
        self.assertIn("published_at TIMESTAMPTZ", self.sql)

    def test_migration_contains_no_persistent_business_seed(self):
        self.assertNotIn("INSERT INTO tenants", self.sql)
        self.assertNotIn("INSERT INTO users", self.sql)
        self.assertNotIn("INSERT INTO collects", self.sql)


if __name__ == "__main__":
    unittest.main()
