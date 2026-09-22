# School Collect agent and contributor rules

This file is the shared operating contract for coding agents and contributors. Read it before making repository changes.

Also read the documents relevant to the task:
- `docs/STATUS.md`
- `docs/V2_PLAN.md`
- `docs/ARCHITECTURE.md`
- `docs/DESIGN_SYSTEM.md` for UI work
- `CONTRIBUTING.md`
- `SECURITY.md`

## 1. Product baseline

School Collect v2 is a production system, not an MVP or disposable prototype.

Do not introduce shortcuts with the assumption that tenancy, authorization, audit, recovery, migrations, offline behavior, or observability will be “fixed later.” Implement the smallest valid slice of the final architecture instead.

The target stack is:
- Tauri 2 + React/Vite + strict TypeScript
- Rust/Axum API
- PostgreSQL 18 + SQLx
- ZITADEL OIDC
- NATS JetStream + transactional outbox
- private R2
- SQLite only for local draft/cache/outbox responsibilities

## 2. Data policy

The repository and production migrations must stay free of persistent demo data.

Do not commit:
- fake schools, fake users, sample submissions, or demo business records as permanent seed data;
- generated fixture databases or test dumps;
- real school work documents or derived copies;
- student, staff, guardian, or other personal information;
- production credentials, tokens, private URLs, signing keys, or secret material.

Tests may create data at runtime through factories/builders and must clean it up with transaction rollback, truncation, disposable databases/containers, or equivalent teardown.

Domain-required reference data is allowed only when it is actual product data rather than demonstration content.

If a legacy file may contain real work information or secrets, do not quote it into issues, logs, PR bodies, or documentation. Treat deletion/history rewriting as a separately authorized security operation.

## 3. Trust boundaries

The Tauri application is an untrusted client.

- Clients call the API; they never receive direct PostgreSQL credentials.
- Server-side authorization is authoritative. UI visibility checks do not grant access.
- Tenant identity, roles, and resource ownership supplied by a client are inputs to verify, not facts to trust.
- Server credentials never ship in frontend bundles, native clients, test fixtures, logs, screenshots, or Figma.
- Runtime DB roles must not perform schema migrations.
- Security-sensitive state must use appropriately protected platform/server storage.

## 4. Architecture discipline

Keep dependency direction explicit.

- `crates/domain`: business invariants; no HTTP/UI/database dependency.
- `crates/application`: server use-cases and ports.
- `crates/db`: SQLx persistence and transaction boundaries.
- `crates/contracts`: API/OpenAPI contracts.
- `crates/auth`: authentication/authorization boundary helpers.
- `services/api`: HTTP composition and request boundary.
- `services/worker`: asynchronous consumers.
- `services/migrator`: migration-only execution.
- `apps/app`: product client and narrow native adapters.
- `packages/ui`: reusable design-system implementation; no business API calls.

v2 migrations live under `migrations/v2/`. Do not make v2 migration execution consume legacy v1 migrations.

Prefer a modular monolith plus explicit workers over premature service splitting.

## 5. Design System rules

The approved Figma Design System is the source of design intent. Product UI must not independently invent a second visual language.

Current approved rules include:
- light compact sidebar; no vertical active indicator;
- one clear primary action per screen;
- radius <= 8px;
- no decorative shadows;
- Card = independent information/action object;
- card area proportional to information/action density;
- do not promote a single peer item into a Card only for emphasis;
- use shared `List Surface + List Row` when a peer collection needs stronger grouping;
- express urgency through semantic color/status/typography;
- design Default/Loading/Empty/Error/Permission/Offline states.

When a product change creates a reusable visual rule, update the Design System contract/component before or together with product usage.

## 6. Git workflow

- Start normal work from `develop` on a short-lived branch.
- Open a PR to `develop`; do not directly push/merge to long-lived branches.
- Feature/fix/refactor/chore/docs -> develop uses squash merge.
- develop -> main uses a merge commit after the exact integration SHA is verified.
- After promotion, sync main -> develop with a merge commit.
- Emergency `hotfix/*` branches start from main and are synced back to develop.
- Never force-push or rewrite shared history unless the owner explicitly authorizes the exact security/history operation.

Branch protection/rulesets must be verified as actual GitHub settings. Documentation alone is not protection.

## 7. Validation and reporting

Run the checks that match the changed area and record what actually ran.

Foundation checks:
```sh
python3 -m unittest discover -s scripts/tests -v
python3 scripts/check_repository.py
```

v2 execution checks are expected to include:
- TypeScript typecheck/build
- Rust fmt/clippy/tests
- PostgreSQL migration/readiness tests
- native Tauri build/smoke
- later: auth/tenant/storage/job/offline integration and Collect E2E

Do not report a scaffold, mock, document, generated config, or passing unrelated check as proof that a subsystem is production-ready.

## 8. Operations requiring explicit authorization

Do not perform any of the following as an incidental side effect:
- production deployment or production migration;
- DNS, domain, certificate, signing-key, or paid infrastructure changes;
- credential rotation in external systems;
- destructive data operations;
- Git history rewrite/force push;
- deletion of legacy business-source documents before the approved retention/security decision.

## 9. Legacy v1

Existing v1 web files are legacy. Change them only when a task explicitly targets v1 maintenance or security cleanup.

Do not copy legacy architecture into v2 merely for compatibility. Preserve useful domain knowledge, not retired infrastructure decisions.
