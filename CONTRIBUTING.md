# 공동작업 규칙

School Collect는 public 오픈소스 저장소를 전제로 작업합니다. 변경 내용뿐 아니라 커밋·PR·로그·스크린샷도 외부 공개 가능한 정보만 포함해야 합니다.

## 브랜치

`main`은 검증된 승격 브랜치, `develop`은 다음 릴리스의 통합 브랜치입니다. 일반 작업은 `develop`에서 분기합니다.

```sh
git fetch origin
git switch develop
git pull --ff-only origin develop
git switch -c feat/collect-form
```

허용 접두사: `feat/`, `fix/`, `refactor/`, `chore/`, `docs/`, `test/`, `build/`, `ci/`, `perf/`.

| PR | 방식 | 조건 |
| --- | --- | --- |
| 작업 브랜치 -> develop | Squash merge | 목적 하나, required CI, 독립 리뷰 |
| develop -> main | Merge commit | 정확한 통합 후보 SHA 검증, 릴리스 승인 |
| main -> develop | Merge commit | 승격/긴급 수정 후 이력 동기화 |
| hotfix/* -> main | Merge commit | main에서 분기, 독립 검증, 이후 main -> develop |

장기 브랜치 사이를 squash/rebase하지 않습니다. 기능이 준비되지 않았으면 합치지 않거나, production 구조를 해치지 않는 검증된 feature flag로 비활성화합니다.

## 리뷰와 권한

PR 작성자 외 최소 1명의 리뷰를 원칙으로 합니다. 인증, DB, tenant isolation, migration, 디자인 토큰, CI, 인프라 변경은 CODEOWNERS 검토 대상입니다.

문서·CODEOWNERS는 GitHub 보호 설정 자체가 아닙니다. 실제 ruleset/branch protection 적용 여부는 [docs/BRANCH_PROTECTION.md](docs/BRANCH_PROTECTION.md)에 기록합니다. 검사 회피용 관리자 bypass나 force push는 금지합니다.

## Production-first 변경 원칙

“초기 버전이니 임시로”라는 이유로 최종 신뢰 경계나 데이터 모델을 우회하지 않습니다. 작은 PR은 환영하지만, 작은 PR도 최종 구조의 일부여야 합니다.

특히:
- client -> API -> DB 경계를 지킵니다.
- authorization을 클라이언트에 두지 않습니다.
- runtime DB 역할과 migration 역할을 분리합니다.
- 파괴적 schema 변경에는 보존/복구 계획을 둡니다.
- DB 변경과 비동기 publish가 함께 필요한 경우 transactional outbox를 기준으로 합니다.

## 데이터와 테스트

production migration에는 가상 학교, 가상 사용자, 예제 업무, 예제 제출 같은 데모 레코드를 넣지 않습니다.

테스트 데이터는 테스트가 필요로 하는 범위에서 실행 중 생성합니다. 권장 방식:
- transaction rollback
- 테스트별 schema/database
- disposable PostgreSQL container
- 명시적 teardown/truncate

테스트가 끝난 뒤 데이터가 남는 구조를 기본값으로 삼지 않습니다. 저장소에 fixture DB dump, 실제처럼 채운 seed, 업무 원본 복사본을 보존하지 않습니다.

도메인상 반드시 필요한 reference data는 데모 레코드와 구분하여 migration으로 관리할 수 있습니다.

## PR 단위

PR 본문에는 다음을 적습니다.
- 관련 Issue/단계
- 변경 이유와 범위
- 실제 실행한 테스트와 결과
- DB/API/auth/tenant 영향
- 환경변수/인프라 영향
- Figma/token/component 영향
- rollback/복구 방법
- 미검증 범위

구조 변경과 대규모 포맷 변경을 기능 PR에 섞지 않습니다.

## 검증

Foundation:

```sh
python3 -m unittest discover -s scripts/tests -v
python3 scripts/check_repository.py
```

v2:
- TypeScript typecheck/build
- Rust fmt/clippy/test
- PostgreSQL migration/readiness
- Windows Tauri native build/smoke
- 이후 auth/tenant/storage/job/offline integration
- Collect E2E

실행하지 않은 검사를 성공으로 적지 않습니다.

패키지 매니저는 pnpm 하나를 사용합니다. Node/Rust/pnpm/crate/npm 버전은 lockfile과 CI를 통해 재현 가능하게 고정합니다.

## DB와 API

v2 migration 기준 경로는 `migrations/v2/`입니다. 이미 배포된 migration을 덮어쓰지 않고 새 migration을 추가합니다. runtime DB 계정으로 DDL을 수행하지 않습니다.

서버가 업무 규칙과 권한의 최종 판단자입니다. 클라이언트는 API만 호출하며 DB credential을 갖지 않습니다. OpenAPI 계약을 중심으로 서버/클라이언트 타입 중복을 줄입니다.

## 디자인

승인된 Figma Design System을 기준으로 구현합니다. 제품 화면에서 반복 가능한 새 패턴이 생기면 Design System 반영 여부를 함께 판단합니다.

현재 핵심 계약:
- radius <= 8px
- decorative shadow 없음
- selective Card usage
- content/action density에 비례한 card sizing
- peer collection 내 단독 Card 승격 금지
- List Surface + uniform List Row
- semantic state로 urgency 표현

## 공개 저장소 보안

실제 업무자료, 개인정보, credential, private endpoint, secret은 Issue/PR/commit/log/screenshot에 넣지 않습니다.

legacy history에 이미 존재하는 민감 가능 자료를 발견하면 값을 재게시하지 말고 [SECURITY.md](SECURITY.md)의 절차를 따릅니다.
