# ADR-0002: Supabase Auth and PostgreSQL as the v2 identity and database providers

Date: 2026-09-25
Status: accepted

## Context

ADR-0001 selected ZITADEL for OIDC and left the hosting location open. That work
never produced a running identity provider. In the meantime the owner had
already registered a Supabase project and an Infisical project (`school collect`,
Andong ICT organization) holding `NEXT_PUBLIC_SUPABASE_URL`,
`NEXT_PUBLIC_SUPABASE_ANON_KEY`, `SUPABASE_SERVICE_ROLE_KEY`, `DATABASE_URL`, and
`SUPABASE_POOLER_URL`.

The registered Supabase project already provides:

- email/password sign-in with confirmed addresses;
- OIDC discovery at `<url>/auth/v1/.well-known/openid-configuration`;
- a JWKS endpoint publishing an ES256 (P-256) signing key;
- PostgreSQL 17 with a direct and a pooler connection string.

## Decision

School Collect v2 uses Supabase Auth as the identity provider and the Supabase
PostgreSQL instance as the system of record.

- The desktop client signs in against Supabase Auth with the public anon key and
  sends the resulting access token to the API as `Authorization: Bearer`.
- The API verifies the token itself (OIDC discovery plus JWKS, `iss`, `aud`,
  signature, expiry). It never trusts a client-supplied identity, tenant, or role.
- `SUB` is linked to `school_collect.users (issuer, subject)`; the tenant and role
  always come from `school_collect.memberships`.
- v2 objects live in the dedicated `school_collect` schema, because the retired
  v1 tables still exist in `public` and one of them (`collect_submissions`) has a
  different shape.
- RLS is not the authorization mechanism. The API is the authority; row security
  remains available later as defense-in-depth.

ZITADEL-specific work is dropped. This supersedes the identity-provider choice in
ADR-0001 only; the rest of that ADR still stands.

## Consequences

- The API binds to the registered project instead of waiting for new
  infrastructure, so the product can be exercised end to end immediately.
- `APP_AUTH_MODE=oidc` is required outside development and `APP_ENV=development`
  alone may use the explicit `disabled` mode. Production-like environments fail
  closed when OIDC configuration is missing.
- The dev, staging, and production Infisical labels currently resolve to the
  **same** Supabase project. Environment separation is therefore not achieved by
  configuration yet; a separate staging and production project remains an open
  owner decision.
- The client still holds no database credential. Only the API and the migrator
  receive `DATABASE_URL`.
- Access tokens are kept in memory only. Persisting a session requires an OS
  keychain-backed adapter and is not part of this decision.

## Verification

`services/api/tests/collect_flow_e2e.rs` performs a real password sign-in against
the registered project, verifies the issued ES256 token through JWKS, and drives
the full Collect flow against the real database, including cross-tenant refusal,
optimistic version conflicts, and cleanup of every record it creates.
