# School Collect

`AGENTS.md` is the shared source of repository instructions for coding agents.

Before implementation, read:
- `AGENTS.md`
- `docs/STATUS.md`
- `docs/V2_PLAN.md`
- `docs/ARCHITECTURE.md`
- `CONTRIBUTING.md`
- `SECURITY.md`
- `docs/DESIGN_SYSTEM.md` for UI work

Critical constraints:
- Treat v2 as production software, not an MVP.
- Do not add persistent demo/fake business data to production migrations or the repository.
- Tests may create temporary data only when they clean it up.
- Supabase is retired and must not be restored as a v2 dependency.
- Server-side authorization is authoritative.
- The approved Figma Design System is the UI baseline.
- Never describe planned/scaffolded infrastructure as deployed or production-ready.
- Never expose legacy secrets or real work data while documenting security cleanup.
