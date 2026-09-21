# 보안 정책

## 저장소와 보고

코드 저장소에는 실제 학생·교직원·학부모 데이터, 업무 원본, 인증 토큰, DB 비밀번호, 서명키를 넣지 않습니다. 테스트는 합성 데이터만 사용합니다. 의심 자격증명이나 개인정보를 발견하면 값/본문을 공개 Issue나 PR에 복사하지 마세요. 소유자에게 기존에 합의된 비공개 경로로 알리고, GitHub private vulnerability reporting은 실제 활성화 여부를 확인한 뒤 사용합니다.

현재 환경변수 예제 정리는 이력 삭제나 credential 무효화를 뜻하지 않습니다. `.gitignore`도 이미 추적 중인 파일에는 소급 적용되지 않습니다. 상세 상태는 `docs/SECURITY_BASELINE.md`에 기록합니다.

## v2 신뢰 경계

- 배포되는 Tauri/Rust 바이너리는 서버가 아닙니다. 앱에 서버 비밀값을 포함하지 않습니다.
- OIDC 로그인은 외부 브라우저 + PKCE, redirect/state/nonce 검증. API는 토큰 서명, issuer, audience, 만료를 검증합니다.
- 학교 선택은 클라이언트 입력일 뿐입니다. 서버 membership 확인과 DB RLS로 학교별 접근을 제한합니다.
- runtime DB 역할은 superuser, BYPASSRLS, table owner가 아니어야 합니다. connection pool의 tenant context는 트랜잭션 범위를 넘기지 않습니다.
- R2는 private bucket을 기본으로 하고 권한 확인 이후 짧은 수명의 파일 URL을 발급합니다.
- 로컬 초안/세션은 사용자·학교별 분리, OS 보호 저장소, 보존 기한, 로그아웃 정리 정책을 적용합니다.
- audit/log에는 암호, 토큰, 학생 원문, 불필요한 before/after 전체 payload를 기록하지 않습니다.
- Tauri capability/CSP/IPC 범위를 최소화하고 운영 빌드에 테스트용 native driver를 포함하지 않습니다.

## 운영 승인

인프라 생성, 실제 서버 접속, DNS 변경, production migration, 인증서/서명키 배포, Git 이력 재작성은 구체적 대상과 권한이 확인된 별도 변경으로 취급합니다. PR 생성만으로 운영 승인을 받은 것으로 보지 않습니다. 백업은 복원 테스트까지 통과해야 합니다.
