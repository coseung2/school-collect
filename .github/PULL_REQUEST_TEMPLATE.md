## 관련 Issue / 단계

## 변경 이유와 범위

## 실제 실행한 테스트와 결과

명령, 대상 OS/DB, 결과를 기재합니다. 실행하지 않은 테스트는 미실행으로 표시합니다.

## 영향
- DB/API 계약:
- 인증/tenant/권한/민감정보:
- migration/data lifecycle:
- 환경변수/인프라/앱 서명:
- Figma node/token/component:

## 테스트 데이터
- 테스트가 생성한 데이터:
- cleanup 방식(rollback/truncate/disposable DB 등):
- repository/production에 남는 demo seed 없음: 예 / 아니오

## 되돌리기 / 데이터 보존

## 확인
- [ ] 목적 하나의 PR이며 대상 브랜치가 올바름
- [ ] 실제 업무자료·개인정보·credential 미포함
- [ ] production migration에 demo seed를 추가하지 않음
- [ ] 필요한 migration/문서/테스트 포함
- [ ] 실제 테스트와 미검증 범위를 구분함
- [ ] 작성자 외 리뷰 및 해당 CODEOWNERS 검토 요청
- [ ] reusable UI 변경은 Design System 영향까지 검토함

릴리스 PR은 검증한 develop SHA, 통합/E2E/native 결과, 배포 승인과 main -> develop 동기화 계획을 추가합니다.
