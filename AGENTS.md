# School Collect contributor and agent rules

Read `docs/STATUS.md`, `docs/V2_PLAN.md`, `CONTRIBUTING.md`, and `SECURITY.md` before changes.

- Work on a short-lived branch based on develop; open a PR to develop. Do not push or merge directly to main/develop.
- The v2 target is Tauri/React + Rust/Axum/PostgreSQL. Existing Next.js files are legacy, not a working v2 implementation.
- The old Supabase project was retired. Do not reconnect, rotate, recreate, or provision it.
- Never put server credentials in the Tauri binary, frontend, fixtures, logs, or Figma.
- Do not delete business source documents, rewrite Git history, provision paid infrastructure, or deploy production without a separately identified and authorized operation.
- Keep authoritative permissions and business invariants on the server. UI checks are not authorization.
- Changes to auth, tenancy, migrations, tokens, native capabilities, CI or infra need owner review.
- Record tests actually executed and blockers. Do not mark a mock, plan, config or scaffold as a deployed system.
- Run `python3 -m unittest discover -s scripts/tests -v` and `python3 scripts/check_repository.py` for foundation changes.
- Figma file creation requires the selected team/file; do not guess between connected teams.

## Legacy Next.js work only

This version may differ from prior knowledge. Before changing legacy Next.js implementation, read the relevant installed documentation in `node_modules/next/dist/docs/` and heed deprecations. New v2 frontend work belongs to the Tauri/Vite workspace after its foundation PR.
