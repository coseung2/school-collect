# 공동작업 규칙

## 브랜치

`main`은 검증된 배포 후보, `develop`은 다음 릴리스의 통합 브랜치입니다. 작업 브랜치는 `develop`에서 분기합니다.

```sh
git fetch origin
git switch develop
git pull --ff-only origin develop
git switch -c feat/collect-form
```

허용 접두사: `feat/`, `fix/`, `refactor/`, `chore/`, `docs/`, `test/`, `build/`, `ci/`, `perf/`.

| PR | 방식 | 조건 |
| --- | --- | --- |
| 작업 브랜치 -> develop | Squash merge | 목적 하나, CI, 독립 리뷰 |
| develop -> main | Merge commit | 통합 후보 SHA 검증, 릴리스 승인 |
| main -> develop | Merge commit | 승격/긴급 수정 후 이력 동기화 |
| hotfix/* -> main | Merge commit | main에서 분기, 독립 검증, 이후 main -> develop |

장기 브랜치 사이를 squash/rebase하지 않습니다. 승격 검증 중 develop에 변경이 생기면 후보 SHA를 다시 검증합니다. 기능이 덜 끝났으면 해당 PR을 합치지 않거나 검증된 feature flag로 비활성화합니다. `main`에 linear history를 강제하지 않습니다.

## 리뷰 및 권한

PR 작성자 외 최소 1명의 리뷰를 원칙으로 합니다. 인증·DB·테넌트 격리·디자인 토큰·CI·인프라 변경은 CODEOWNERS 리뷰 대상입니다. 헤드가 작성한 PR은 자기 승인으로 대체하지 않고 지정된 다른 리뷰어가 검토합니다. 대리 리뷰어가 없는 동안 리뷰/승격 게이트는 미완료로 둡니다.

CODEOWNERS와 이 문서는 GitHub 보호 규칙 자체가 아닙니다. 실제 강제 설정은 [별도 체크리스트](docs/BRANCH_PROTECTION.md)로 확인합니다. 검사 회피를 위한 관리자 bypass나 force-push는 금지합니다.

## PR 단위

Issue, 변경 이유, 범위, 테스트 결과, DB/환경변수/보안/디자인 영향, 되돌리기 방법을 적습니다. 구조 변경과 대규모 포맷 변경을 기능 PR에 섞지 않습니다. 변경된 Figma 노드와 코드 token/variant의 대응을 남깁니다. 실제 업무자료를 스크린샷·본문·테스트에 올리지 않습니다.

## 검증

현재 foundation 검사:

```sh
python3 -m unittest discover -s scripts/tests -v
python3 scripts/check_repository.py
```

3단계부터 TS lint/typecheck/test/build 및 Rust fmt/clippy/test, DB migration 테스트를 추가합니다. 5단계부터 native build/smoke, 6단계부터 auth/tenant/storage/job/offline 통합, 7단계부터 실제 Collect E2E가 승격 조건입니다. 아직 없는 검사를 성공으로 표시하지 않습니다.

패키지 매니저는 pnpm 하나를 사용합니다. 버전은 저장소 선언과 lockfile을 따르고 신규 lockfile 생성을 다른 매니저로 하지 않습니다. Rust/Node/시스템 라이브러리 버전은 재현 가능한 workspace PR에서 함께 고정합니다.

## DB 및 API

v2 migration의 기준 경로는 `db/migrations/`입니다. 이미 통합/배포된 migration은 덮어쓰지 않고 새 migration을 추가합니다. 빈 DB와 이전 버전 DB 모두 검증합니다. runtime DB 계정으로 DDL을 수행하지 않습니다. 파괴적 변경에는 데이터 보존과 복구 계획이 필요합니다.

서버가 업무규칙과 권한의 최종 판단자입니다. 클라이언트는 API만 호출하며 DB credential을 갖지 않습니다. OpenAPI 계약으로 클라이언트를 생성하고 서버/TS 타입을 수동으로 이중 관리하지 않는 것을 목표로 합니다.
