# 보안 정책

School Collect 저장소는 public 오픈소스 운영을 전제로 합니다. 따라서 모든 코드, 문서, commit, PR, CI log는 외부 공개되어도 되는 정보만 포함해야 합니다.

## 저장소에 들어가면 안 되는 것

- 학생·교직원·학부모 등 실제 개인정보
- 실제 학교 업무 원본과 그 복사/파생 파일
- 운영 access token, refresh token, API key, DB password
- signing key, private key, certificate private material
- private endpoint나 내부 운영정보 중 공개가 승인되지 않은 값
- production DB dump
- demo 목적으로 영구 보존되는 가상 업무 seed/fixture DB

production migration은 schema, constraint, index와 실제 제품에 필요한 reference data를 관리합니다. 데모 학교/사용자/업무/제출 데이터는 넣지 않습니다.

테스트 데이터는 테스트 실행 시 생성하고 rollback, truncate, disposable DB/container 등으로 제거합니다.

## legacy history

과거 v1 이력에서 실제처럼 보이는 credential과 업무 원본/파생 artifact가 확인되어 별도 정리 중입니다. 해당 값을 공개 Issue, PR, 문서, 로그에 다시 복사하지 않습니다.

현재 HEAD에서 값을 비우거나 파일을 삭제하는 것만으로 과거 Git object가 제거되지는 않습니다. credential은 외부 시스템에서 폐기/회전되어야 하며, history rewrite가 필요하면 범위·백업·협업자 재동기화·공개 캐시 한계를 확인한 별도 보안 작업으로 수행합니다.

상세 상태: [docs/SECURITY_BASELINE.md](docs/SECURITY_BASELINE.md)

## v2 신뢰 경계

- Tauri 앱은 신뢰 서버가 아닙니다.
- 서버 credential은 frontend/native binary에 포함하지 않습니다.
- OIDC는 external browser + Authorization Code + PKCE를 기준으로 합니다.
- API는 token signature, issuer, audience, expiration을 검증합니다.
- tenant/role/resource ownership은 서버가 membership과 정책으로 검증합니다.
- runtime DB 역할은 schema owner/superuser/BYPASSRLS가 아니어야 합니다.
- migration은 별도 권한/프로세스로 수행합니다.
- private R2 접근은 서버 권한 확인 후 제한된 방식으로 제공합니다.
- audit/log에는 token, password, 불필요한 개인정보 원문, 전체 payload dump를 기록하지 않습니다.
- local draft/session은 사용자·tenant 경계를 유지하고 logout/expiration cleanup 정책을 가집니다.
- Tauri capability/CSP/IPC 권한은 최소화합니다.

## 취약점/비밀정보 보고

민감한 값을 공개 Issue에 게시하지 마세요. 저장소 소유자에게 합의된 비공개 채널을 사용합니다. GitHub private vulnerability reporting을 사용할 경우 실제 활성화 여부를 먼저 확인합니다.

## 운영 변경

다음 작업은 코드 PR과 별개의 명시적 운영 권한을 요구합니다.
- 실제 credential 폐기/회전
- production DB 접속/migration
- DNS/domain/TLS 변경
- production deployment
- 서명키/업데이트키 배포
- paid infrastructure 생성
- Git history rewrite/force push
- 실제 데이터 삭제/보존 변경

백업은 존재 여부가 아니라 **복원 검증**까지 포함해야 합니다.
