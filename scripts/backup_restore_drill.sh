#!/usr/bin/env bash
# C-03 backup/restore drill (repository-side preparation).
#
# Dumps SOURCE_DATABASE_URL in pg_dump custom format, restores it into the
# disposable RESTORE_DATABASE_URL, and proves the restored copy matches the
# source: schema version, applied migrations, table inventory, and per-table
# row counts. Any mismatch fails the drill, so a restore is never assumed to
# be good just because pg_restore exited 0.
#
# The source database is read-only unless DRILL_CANARY=1, which writes one
# temporary tenant row and removes it again on exit.
#
# Required:
#   SOURCE_DATABASE_URL    database to back up
#   RESTORE_DATABASE_URL   disposable database that receives the restore
# Optional:
#   PG_CLIENT_IMAGE        docker image providing pg_dump/pg_restore/psql,
#                          used when the host client major differs from the
#                          server (CI uses postgres:18)
#   DRILL_CANARY=1         write a temporary source row and require it to
#                          survive the round trip
#   DRILL_WORK_DIR         directory for the dump file (default: mktemp -d)
#
# This script is intended for CI and for an operator shell on Linux/macOS.
set -euo pipefail

fail() {
  echo "backup/restore drill failed: $*" >&2
  exit 1
}

[[ -n "${SOURCE_DATABASE_URL:-}" ]] || fail "SOURCE_DATABASE_URL is required"
[[ -n "${RESTORE_DATABASE_URL:-}" ]] || fail "RESTORE_DATABASE_URL is required"

work_dir="${DRILL_WORK_DIR:-$(mktemp -d)}"
mkdir -p "$work_dir"
dump_file="$work_dir/school-collect-drill.dump"
client_image="${PG_CLIENT_IMAGE:-}"

# The client container may run as another uid than the calling user; the
# directory only holds this drill's dump file and is removed again.
if [[ -n "$client_image" ]]; then
  chmod 777 "$work_dir"
fi

# Credentials never reach the log; only the shape of the target is printed.
describe_url() {
  printf '%s' "$1" | sed -E 's#^([a-z]+)://[^@/]*@#\1://<credentials>@#'
}

# CI runs the PostgreSQL 18 clients from the postgres image so pg_dump and
# pg_restore always match the server major version. `--network host` lets
# those containers reach a database published on 127.0.0.1 (Linux). Stdin is
# never attached: the client must not consume the caller's input.
client() {
  if [[ -n "$client_image" ]]; then
    docker run --rm --network host -v "$work_dir:/drill" -w /drill \
      "$client_image" "$@" </dev/null
  else
    "$@"
  fi
}

client_path() {
  if [[ -n "$client_image" ]]; then
    printf '/drill/%s' "$(basename "$1")"
  else
    printf '%s' "$1"
  fi
}

query() {
  client psql "$1" -v ON_ERROR_STOP=1 -tAc "$2"
}

canary_id="00000000-0000-4000-8000-0000000000c0"
canary_created=0

cleanup() {
  if [[ "$canary_created" == "1" ]]; then
    query "$SOURCE_DATABASE_URL" \
      "delete from school_collect.tenants where id = '$canary_id'" >/dev/null 2>&1 || true
    query "$RESTORE_DATABASE_URL" \
      "delete from school_collect.tenants where id = '$canary_id'" >/dev/null 2>&1 || true
  fi
  if [[ -z "${DRILL_WORK_DIR:-}" ]]; then
    rm -rf "$work_dir"
  fi
}
trap cleanup EXIT

source_db=$(query "$SOURCE_DATABASE_URL" "select current_database()") \
  || fail "cannot connect to SOURCE_DATABASE_URL"
restore_db=$(query "$RESTORE_DATABASE_URL" "select current_database()") \
  || fail "cannot connect to RESTORE_DATABASE_URL; create the disposable database first"
[[ "$source_db" != "$restore_db" ]] \
  || fail "SOURCE_DATABASE_URL and RESTORE_DATABASE_URL point at the same database"

source_version=$(query "$SOURCE_DATABASE_URL" \
  "select schema_version from public.app_meta where singleton_id = 1") \
  || fail "source database has no app_meta row; run the migrator first"
[[ -n "$source_version" ]] || fail "source database app_meta.schema_version is empty"

echo "source:  $(describe_url "$SOURCE_DATABASE_URL") (schema_version=$source_version)"
echo "restore: $(describe_url "$RESTORE_DATABASE_URL")"

if [[ "${DRILL_CANARY:-}" == "1" ]]; then
  query "$SOURCE_DATABASE_URL" \
    "insert into school_collect.tenants (id, name)
     values ('$canary_id', 'backup-restore-drill-canary')
     on conflict (id) do nothing" >/dev/null
  canary_created=1
fi

echo "dumping to $(client_path "$dump_file")"
start=$SECONDS
client pg_dump --format=custom --no-owner --no-privileges \
  --file "$(client_path "$dump_file")" "$SOURCE_DATABASE_URL"
dump_seconds=$((SECONDS - start))
[[ -s "$dump_file" ]] || fail "pg_dump produced no dump file"

echo "restoring"
start=$SECONDS
client pg_restore --exit-on-error --clean --if-exists --no-owner --no-privileges \
  --dbname "$RESTORE_DATABASE_URL" "$(client_path "$dump_file")"
restore_seconds=$((SECONDS - start))

echo "verifying the restored copy"
restored_version=$(query "$RESTORE_DATABASE_URL" \
  "select schema_version from public.app_meta where singleton_id = 1")
[[ "$source_version" == "$restored_version" ]] \
  || fail "schema_version differs: source=$source_version restore=$restored_version"

source_migrations=$(query "$SOURCE_DATABASE_URL" \
  "select count(*) from public._sqlx_migrations where success")
restored_migrations=$(query "$RESTORE_DATABASE_URL" \
  "select count(*) from public._sqlx_migrations where success")
[[ "$source_migrations" == "$restored_migrations" ]] \
  || fail "applied migration count differs: source=$source_migrations restore=$restored_migrations"

table_list="$work_dir/table-list.txt"
query "$SOURCE_DATABASE_URL" \
  "select table_name from information_schema.tables
   where table_schema = 'school_collect' order by table_name" > "$table_list"
[[ -s "$table_list" ]] \
  || fail "source database has no school_collect tables; run the migrator first"
source_tables=$(cat "$table_list")
restored_tables=$(query "$RESTORE_DATABASE_URL" \
  "select table_name from information_schema.tables
   where table_schema = 'school_collect' order by table_name")
[[ "$source_tables" == "$restored_tables" ]] \
  || fail "school_collect table inventory differs between source and restore"

checked_tables=0
while IFS= read -r table_name; do
  [[ -n "$table_name" ]] || continue
  source_rows=$(query "$SOURCE_DATABASE_URL" \
    "select count(*) from school_collect.\"$table_name\"")
  restored_rows=$(query "$RESTORE_DATABASE_URL" \
    "select count(*) from school_collect.\"$table_name\"")
  [[ "$source_rows" == "$restored_rows" ]] \
    || fail "row count differs for school_collect.$table_name: source=$source_rows restore=$restored_rows"
  checked_tables=$((checked_tables + 1))
done < "$table_list"

if [[ "$canary_created" == "1" ]]; then
  canary_rows=$(query "$RESTORE_DATABASE_URL" \
    "select count(*) from school_collect.tenants
     where id = '$canary_id' and name = 'backup-restore-drill-canary'")
  [[ "$canary_rows" == "1" ]] || fail "canary row did not survive the restore"
fi

echo "backup/restore drill OK: schema_version=$source_version migrations=$source_migrations tables=$checked_tables canary=$canary_created dump_bytes=$(wc -c < "$dump_file") dump_seconds=$dump_seconds restore_seconds=$restore_seconds"
