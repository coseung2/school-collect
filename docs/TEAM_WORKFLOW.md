# 팀 기능 개발 운영 기준

이 문서는 `develop`에서 기능 브랜치를 만들어 작업하는 팀원이 따라야 할 공통 기준입니다.

## 시작

```sh
git fetch origin
git switch develop
git pull --ff-only origin develop
git switch -c feat/collect-draft
pnpm install --frozen-lockfile
```

기능 브랜치는 `feat/`, `fix/`, `refactor/`, `test/`, `docs/`, `build/`, `ci/`, `perf/` 중 하나의 접두사를 사용합니다. `main`과 `develop`에서 직접 작업하지 않습니다.

## 한 브랜치의 소유 범위

기능은 한 사용자 흐름의 작은 vertical slice로 나눕니다.

- UI: `apps/app`, `packages/ui`
- HTTP/API: `services/api`, `crates/contracts`
- 업무 규칙: `crates/domain`, `crates/application`
- DB: `crates/db`, `migrations/v2`
- 비동기 작업: `services/worker`
- 인증·권한: `crates/auth`

DB schema, API contract, domain state를 바꾸는 PR은 변경 순서와 영향 범위를 본문에 적습니다. 다른 팀원의 기능을 위한 공용 계약은 먼저 별도 PR로 확정합니다.

## 현재 공용 계약

- 모든 업무 데이터는 `tenant_id`를 가집니다.
- 서버가 OIDC subject와 membership에서 actor와 role을 결정합니다.
- client가 보낸 tenant, role, ownership은 검증 대상입니다.
- Collect 상태는 `draft -> published -> closed` 순서로만 이동합니다.
- 재시도 가능한 mutation은 version 또는 idempotency 기준을 가져야 합니다.
- DB 변경과 outbox event는 같은 transaction에서 commit합니다.
- production migration과 저장소에는 demo 업무 데이터를 넣지 않습니다.

세부 API/DB 계약은 [API_CONTRACT.md](API_CONTRACT.md)와 `migrations/v2/`를 기준으로 합니다.
팀별 소유 경로와 선행 조건은 [TEAM_BACKLOG.md](TEAM_BACKLOG.md)를 기준으로 합니다.

## PR 완료 기준

- 목적과 소유 범위가 하나입니다.
- 관련 migration은 forward-only입니다.
- 인증, tenant isolation, role, 상태 전이를 서버 테스트로 검증합니다.
- `python3 -m unittest discover -s scripts/tests -v`
- `python3 scripts/check_repository.py`
- 변경 영역에 맞는 `pnpm`/`cargo` 검사를 실행합니다.
- 실행하지 못한 검사는 실패로 기록하고 성공으로 표현하지 않습니다.
- PR 본문에 rollback 방법과 남은 검증 범위를 적습니다.
