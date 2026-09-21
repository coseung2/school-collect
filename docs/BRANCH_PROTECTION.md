# main / develop 보호 설정 체크리스트

상태: 적용 전. 문서·CODEOWNERS·workflow 생성은 GitHub 서버의 보호 설정 적용과 다릅니다. 현재 연결된 GitHub App의 administration 권한과 지원 작업을 확인한 뒤 실제 설정 결과를 기록해야 합니다.

## 공통

PR required, 작성자 외 approval >= 1, stale review 해제, conversation resolution, code owner review, force push/삭제 차단을 적용합니다. main/develop 직접 push 및 상시 관리자 bypass를 금지합니다. squash와 merge commit 둘 다 repository에서 허용해야 합니다. rebase merge는 운영 규칙에서 사용하지 않습니다.

## develop

초기 필수 체크는 실제 성공을 확인한 `repository-checks`입니다. 기능 브랜치는 squash merge. main -> develop 동기화에는 merge commit이 필요하므로 develop에도 무조건적인 linear history 요구를 걸지 않습니다.

## main

동일 저장소 develop 승격 또는 main 기반 hotfix만 허용합니다. branch-direction CI는 보조 게이트이고 관리자/규칙 우회를 차단하는 설정의 대체재는 아닙니다. 승격은 merge commit을 사용하므로 linear history 필수 설정을 켜지 않습니다.

후속 필수 검사: TS lint/typecheck/test/build, Rust fmt/clippy/test, PostgreSQL migration/RLS/API integration, native build/smoke, Collect E2E. 존재하지 않거나 아직 실행되지 않은 check 이름을 미리 필수로 등록하지 않습니다.

## 적용 검증

- [ ] main/develop 직접 push 차단
- [ ] feature -> main PR 방향 검사 실패
- [ ] CI 실패/미완료 상태에서 merge 차단
- [ ] auth/db/token/CI 변경 시 owner review 요청
- [ ] head 본인 PR의 별도 리뷰어 지정
- [ ] develop -> main merge commit 후 main -> develop 동기화 시험
- [ ] hotfix 경로 검증

GitHub App 연결은 repository metadata가 admin으로 보이더라도 administration API 권한이 없을 수 있습니다. 적용하지 못한 항목을 적용 완료로 기록하지 않습니다.

공식 문서: https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches
