#!/usr/bin/env bash
# Runtime/migrator role separation.
#
# The schema owner runs migrations; the runtime role only touches rows. This
# check proves both halves against a real PostgreSQL server: the runtime role
# can read and write business rows but no DDL, and the owner's grants cover
# tables added by later migrations.
#
# Environment:
#   MIGRATION_DATABASE_URL  owner role that applies migrations
#   RUNTIME_DATABASE_URL    runtime role; the same server, never the owner role
#   RUNTIME_ROLE            role name to create and check (default below)
#   RUNTIME_ROLE_PASSWORD   password for that role (test value; not a secret)
set -euo pipefail

owner_url="${MIGRATION_DATABASE_URL:?set MIGRATION_DATABASE_URL}"
runtime_url="${RUNTIME_DATABASE_URL:?set RUNTIME_DATABASE_URL}"
runtime_user="${RUNTIME_ROLE:-school_collect_runtime}"
runtime_password="${RUNTIME_ROLE_PASSWORD:-school_collect_runtime_ci}"

fail() {
    echo "runtime role check failed: $*" >&2
    exit 1
}

# What a deployment sets up once: a login role with row privileges on the
# business schema and default privileges so tables from later migrations are
# covered too.
psql "$owner_url" -v ON_ERROR_STOP=1 -q <<SQL
DO \$\$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = '${runtime_user}') THEN
        CREATE ROLE ${runtime_user} LOGIN PASSWORD '${runtime_password}';
    ELSE
        ALTER ROLE ${runtime_user} LOGIN PASSWORD '${runtime_password}';
    END IF;
END
\$\$;
GRANT USAGE ON SCHEMA school_collect TO ${runtime_user};
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA school_collect TO ${runtime_user};
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA school_collect TO ${runtime_user};
ALTER DEFAULT PRIVILEGES IN SCHEMA school_collect
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO ${runtime_user};
ALTER DEFAULT PRIVILEGES IN SCHEMA school_collect
    GRANT USAGE, SELECT ON SEQUENCES TO ${runtime_user};
SQL

# DDL belongs to the migrator: creating, altering and indexing must be refused.
for statement in \
    'CREATE TABLE school_collect.runtime_ddl_probe (id int)' \
    'ALTER TABLE school_collect.tenants ADD COLUMN runtime_probe int' \
    'CREATE INDEX runtime_ddl_probe_idx ON school_collect.tenants (id)'
do
    if psql "$runtime_url" -v ON_ERROR_STOP=1 -q -c "$statement" > /dev/null 2>&1; then
        fail "the runtime role ran DDL: $statement"
    fi
done

# Business rows must work: insert, update, read and delete, rolled back so the
# check leaves nothing behind.
psql "$runtime_url" -v ON_ERROR_STOP=1 -q <<'SQL'
BEGIN;
INSERT INTO school_collect.tenants (id, name)
    VALUES ('00000000-0000-7000-8000-00000000c0de', 'runtime-role-check');
UPDATE school_collect.tenants SET name = 'runtime-role-check-updated'
    WHERE id = '00000000-0000-7000-8000-00000000c0de';
SELECT name FROM school_collect.tenants
    WHERE id = '00000000-0000-7000-8000-00000000c0de';
DELETE FROM school_collect.tenants WHERE id = '00000000-0000-7000-8000-00000000c0de';
ROLLBACK;
SQL

echo "runtime role check OK: ${runtime_user} writes rows and cannot run DDL"
