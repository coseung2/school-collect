# main / develop 보호 설정 체크리스트

기준일: 2026-09-22.

## 현재 확인 상태

- repository rulesets API 조회 결과: `[]`
- classic branch protection read: 현재 연결의 administration 권한 부족으로 403
- 따라서 main/develop 보호가 활성화되었다고 주장하지 않음

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

- [ ] main 직접 push 차단
- [ ] develop 직접 push 차단
- [ ] feature -> main 차단
- [ ] required CI 실패/대기 상태 merge 차단
- [ ] code-owner path review 동작
- [ ] 작성자 자기 승인만으로 merge 불가
- [ ] force push 차단
- [ ] branch deletion 차단
- [ ] develop -> main merge commit
- [ ] main -> develop sync
- [ ] hotfix -> main -> develop 경로 검증

연결된 GitHub App이 metadata상 admin으로 보여도 branch administration API 권한이 없을 수 있습니다. 실제 UI/관리 권한으로 설정한 결과를 체크리스트에 반영해야 합니다.
