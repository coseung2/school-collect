# main / develop 보호 설정

기준일: 2026-09-22.

## 현재 적용 상태

classic branch protection을 `main`과 `develop`에 적용하고 API 조회로 확인했습니다.

| 항목 | main | develop |
| --- | --- | --- |
| pull request 필수 | 적용 | 적용 |
| stale approval dismissal | 적용 | 적용 |
| conversation resolution 필수 | 적용 | 적용 |
| required status check | `repository-checks` | `repository-checks` |
| 최신 base 강제 (strict) | 적용 | 미적용 |
| 관리자에게도 적용 (enforce_admins) | 적용 | 적용 |
| force push | 차단 | 차단 |
| branch 삭제 | 차단 | 차단 |
| linear history 강제 | 미적용 | 미적용 |

`develop`에 strict를 적용하지 않은 이유는 main -> develop 동기화가 merge commit이고, 다수 PR이 동시에 열려 있을 때 불필요한 재빌드를 강제하지 않기 위해서입니다. `main`은 승격 대상이므로 strict를 적용합니다.

linear history는 두 브랜치 모두 강제하지 않습니다. develop -> main 승격과 main -> develop 동기화가 merge commit이기 때문입니다.

### required check 선정 기준

`repository-checks`만 등록했습니다. 이 job은 `foundation.yml`에 있고 모든 PR에서 실행되므로 required로 안전합니다.

`V2 Architecture` workflow의 job(`rust-and-web`, `postgres`, `lockfiles-verified`, `tauri-windows-smoke`)은 `paths` 필터를 사용합니다. path 조건에 맞지 않는 PR에서는 아예 실행되지 않으므로, required로 등록하면 문서만 수정한 PR이 영구 대기 상태가 됩니다. 이 job들을 required로 만들려면 먼저 path 필터를 제거하거나 skip 시 성공을 보고하는 경로를 만들어야 합니다.

### 미적용 항목

approval 최소 개수는 0입니다. 현재 저장소는 단독 운영이라 1 이상으로 두면 본인 PR을 병합할 수 없습니다. 협업자가 추가되면 1 이상과 code owner review를 함께 올려야 합니다.

문서, CODEOWNERS, CI workflow는 GitHub 서버의 실제 보호 설정을 대신하지 않습니다.

## 공통 목표

main/develop:
- pull request required
- 작성자 외 approval >= 1
- stale approval dismissal
- conversation resolution
- required status checks
- code owner review
- force push 금지
- branch deletion 금지
- 상시 관리자 bypass 금지

repository에는 squash merge와 merge commit 둘 다 필요합니다. rebase merge는 운영 workflow에서 사용하지 않습니다.

## develop

작업 브랜치는 `develop`으로 PR을 열고 squash merge합니다.

required checks는 실제 존재하고 안정적으로 성공한 workflow만 등록합니다. Stage 3 이후에는 repository foundation 검사와 v2 architecture 검사 중 실제 branch path에 적용되는 check를 기준으로 고정합니다.

main -> develop 동기화는 merge commit이므로 develop에 unconditional linear history를 강제하지 않습니다.

## main

허용 경로:
- `develop -> main` release promotion
- `hotfix/* -> main`

feature branch -> main 직접 PR은 허용하지 않습니다.

promotion은 merge commit을 사용합니다. exact develop candidate SHA의 통합 검증이 끝난 뒤 승격합니다.

## 적용 검증

API 조회로 확인한 항목:

- [x] main pull request 필수 + force push/삭제 차단
- [x] develop pull request 필수 + force push/삭제 차단
- [x] required status check `repository-checks` 등록
- [x] enforce_admins로 관리자 bypass 차단
- [x] conversation resolution 필수
- [x] develop -> main merge commit 경로 (PR #12에서 실제 수행)
- [x] main -> develop sync 경로 (PR #12에서 실제 수행)

아직 실제 상황으로 검증하지 않은 항목:

- [ ] required CI 실패/대기 상태에서 merge 차단 동작
- [ ] feature -> main 직접 PR 차단 (현재 protection은 base 브랜치를 제한하지 않으므로 규칙으로만 운영)
- [ ] code-owner path review 동작 (require_code_owner_reviews 미적용)
- [ ] 작성자 자기 승인만으로 merge 불가 (approval 0이라 현재 해당 없음)
- [ ] hotfix -> main -> develop 경로
