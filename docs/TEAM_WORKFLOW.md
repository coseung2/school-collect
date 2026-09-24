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

## 로컬 개발 환경

개발 환경은 공용으로 두지 않습니다. 각자 자기 기계에서 서버 스택을 컨테이너로 띄우므로, 개발 DB와 개발 데이터가 팀원 사이에 섞이지 않습니다.

```sh
docker compose -f infra/compose.dev.yml up -d
pnpm --filter @school-collect/app tauri dev
```

- `postgres`(18)와 `nats`가 뜨고, 일회성 `migrator`가 `migrations/v2`를 적용한 뒤 종료하며, `api`는 그 다음에 시작합니다.
- API는 `http://127.0.0.1:3000`이고 `/health`(생존)와 `/ready`(스키마·DB 포함 준비)를 제공합니다.
- 상태 확인은 `docker compose -f infra/compose.dev.yml ps`, 종료는 `... down`, 개발 데이터까지 초기화하려면 `... down -v`입니다.
- 서버 이미지를 처음 빌드하므로 첫 `up`은 몇 분 걸립니다. 이후에는 캐시를 재사용합니다.
- 클라이언트만 고치는 작업에는 Rust 툴체인이 필요하지 않습니다. 서버는 컨테이너에서 돕니다.

기본값 `APP_AUTH_MODE=disabled`는 `APP_ENV=development`에서만 허용되고, 이때 API는 고정 development 주체로 동작합니다. 실제 로그인 경로까지 확인하려면 `APP_AUTH_MODE=oidc`와 `OIDC_ISSUER_URL`, `OIDC_AUDIENCE`를 지정합니다. `development`가 아닌 환경은 인증을 끈 채로 기동할 수 없습니다.

공용 staging/production 환경은 별도이며, 결정되지 않은 항목은 [DEPLOYMENT.md](DEPLOYMENT.md)에 있습니다.

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
