# 실행 상태

기준일: 2026-09-27 (Asia/Seoul). 이 문서는 계획, 기능 브랜치 구현, develop 통합과 실제 검증을 구분합니다.

작업 분류는 [V2_PLAN.md](V2_PLAN.md), 상세 계획은 [학교업무](SCHOOL_WORK_PLAN.md) / [개인기능](PERSONAL_PLAN.md), 공통 기반은 [TEAM_BACKLOG.md](TEAM_BACKLOG.md)를 따릅니다. `내 제출`과 offline 제출 초안은 학교업무이며 개인 바로가기·자동입력·감시는 개인기능입니다.

## 이번에 확인한 범위

- `git fetch origin` 후 PR #24(S-01, `f573bfb`)와 PR #25(P-01, `11a0af0`), 계획 문서 PR #26(`65058b3`)을 develop에 통합했습니다.
- 두 PR 모두 병합 전 CI 6개(`repository-checks`, `rust-and-web`, `lockfiles-verified`, `postgres`, `server-image`, `tauri-windows-smoke`)가 성공했습니다.
- 병합 후 실제 데스크톱 창에서 로그인 없는 개인 화면 열기, 레시피 등록·열기·삭제·실행 취소·재시작 보존을 확인했고, 학교 화면은 Permission 상태로 표시되는 것을 확인했습니다.
- S-02 초대는 실제 Supabase 로그인과 실제 PostgreSQL에서 E2E(`membership_invitation_end_to_end`)로 확인했고, 생성한 계정·행은 테스트가 삭제했습니다. PR #28(`9416c6c`)로 develop에 통합했습니다.
- P-02 로컬 브리지·브라우저 확장은 PR #30으로 develop에 통합했습니다(`795dc26`). 병합 전 CI 6개가 성공했고, 실제 브라우저(Edge) 검증은 병합 전 브랜치 SHA에서 수행했습니다.
- P-03 자동입력 레시피 엔진은 1차(저장·요소 선택·미리보기, PR #32, `455dcdc`)와 2차(실행·결과 검증, PR #33, `5318a96`)를 모두 develop에 통합했습니다.
- P-04 학생 × 날짜 행렬 입력은 PR #35(`09b3f88`)로 develop에 통합했습니다. 병합 전 CI 6개가 성공했고, 실제 Edge에서 등록·미리보기·입력과 중단 경로 5개를, 실제 Tauri 창에서 표시·두 단계 삭제를 확인했습니다.
- P-06 새 신청·결재 감시는 PR #37(`e5415dc`)로 develop에 통합했습니다. 병합 전 CI 6개가 성공했고, 실제 Edge에서 등록·스캔 3회·중단 2경로를, 실제 Tauri 창에서 새 항목 표시·확인·중지·다시 켜기·삭제와 감시 상태 정리를 확인했습니다.
- C-02 인증·권한·관측 보강은 PR #39(`7285981`)로 develop에 통합했습니다. 병합 전 CI 6개가 성공했고, `postgres` 작업에서 풀 연결 재사용 시 학교 격리 테스트(`tenant_isolation.rs`)가 실제 PostgreSQL로 통과했습니다.
- S-03 항목·대상 편집은 PR #41(`70dcbfe`)로 develop에 통합했습니다. 병합 전 CI 6개가 성공했고, 실제 개발 DB에서 편집 규칙 테스트 2개와 기존 흐름 E2E 2개가 통과했으며, 실제 Tauri 창에서 항목·대상 편집 저장을 확인했습니다.
- S-06 outbox/worker는 PR #43(`e722bce`)로 develop에 통합했습니다. 병합 전 CI 7개(새 `nats-outbox` 포함)가 성공했고, 실제 개발 DB relay 테스트 6개와 JetStream 배달·중복 제거를 실제 브로커로 확인했습니다.
- S-05 세션·오프라인 초안은 PR #45(`10d1c14`)와 PR #46(`93cebdc`)로 develop에 통합했습니다. 병합 전 CI 7개가 성공했고, 실제 Tauri 창에서 세션 저장·재시작 복구·로그아웃 정리와 오프라인 초안 저장·복구·동기화·정리를 확인했습니다. 외부 브라우저 PKCE는 남아 있습니다.
- S-07 결과 내보내기 1차는 PR #48(`9860710`)로 develop에 통합했습니다. 병합 전 CI 7개가 성공했고, 실제 프로젝트 E2E에서 담당자 200(두 구성원 행·쉼표 값 인용·감사 1건)과 교사 403을 확인했습니다. 전체 흐름 검증과 화면 버튼은 남아 있습니다.
- S-07 결과 내보내기 2차(화면 버튼·전체 흐름 검증)는 PR #50(`88b0a38`)으로 develop에 통합했습니다. 병합 전 CI 7개가 성공했고, 실제 Tauri 창에서 수합 상세의 `결과 CSV 내보내기`로 다운로드 폴더에 저장되는 것과 실제 프로젝트 E2E 3개(전체 흐름·초대·내보내기) 통과를 확인했습니다.
- P-07 레시피 추천은 PR #51(`8704d95`)로 develop에 통합했습니다. `apps/extension`과 `scripts/tests`만 바꿔 변경 경로에 해당하는 `repository-checks`가 성공했고, 실제 Edge에서 자동입력·표 입력·감시 후보의 생성·저장·실행과 로그인 화면·화면 변경 중단 경로를 확인했습니다.
- S-07 역할 4종 검증은 PR #53(`9b63abf`)으로 develop에 통합했습니다. 병합 전 CI 7개가 성공했고, 실제 provider + 실제 PostgreSQL에서 `collect_flow_e2e` 4개(전체 흐름·초대·내보내기·역할 4종)를 통합 SHA `9b63abf` 기준으로 다시 통과시켰습니다. 남은 행 0, provider에 `@example.test` 계정 없음을 확인했습니다.
- C-01/C-05 native 검증은 PR #54(`394f9c9`)로 develop에 통합했습니다. 실제 Tauri 창에서 permission·설정·offline·키보드·narrow를 확인해 설정 안내 문구(세션 저장 방식)와 640px 가로 넘침을 고쳤습니다. 병합 전 CI 7개가 성공했습니다.
- S-04 파일 첨부 1차(metadata 계약·보존 규칙)는 PR #58(`e3a541c`)로 develop에 통합했습니다. 병합 전 CI 7개가 성공했고, 실제 개발 DB에서 `crates/db/tests/attachments.rs` 5개(슬롯 열기·완료·목록/조회·삭제 규칙·만료/purge)가 통과했습니다.
- S-04 2차(업로드·다운로드 API와 저장소 port)는 PR #59(`2486f8d`)로 develop에 통합했습니다. 병합 전 CI 7개가 성공했고, 실제 provider+PostgreSQL E2E `attachment_flow_end_to_end`가 업로드·조회·목록·삭제와 다른 학교 접근 404/403을 확인한 뒤 생성한 계정·행을 모두 삭제합니다.
- S-04 3차(보존 기간 정리 sweep)는 PR #60(`aaa55c5`)로 develop에 통합했습니다. 병합 전 CI 7개가 성공했고, 실제 개발 DB에서 worker `attachment_sweep` 1개(만료 첨부 삭제, bytes 삭제 실패 시 행 유지)와 relay 6개가 통과했습니다. 통합 SHA `aaa55c5`에서 E2E 5개와 `@example.test` 잔여 0을 다시 확인했습니다.
- 독립 검토가 저장소 안에서 선행 가능하다고 밝힌 나머지 범위를 PR #62~#66으로 통합했습니다. #62·#63·#65·#66은 병합 전 CI 7개, 문서·스크립트만 바꾼 #64는 해당 경로의 `repository-checks` 1개가 성공했습니다.
  - C-03 백업·복구 drill(PR #62, `74e1149`): CI `postgres` 작업이 dump → 빈 DB 복구 → schema version·migration 5개·`school_collect` 테이블 13개 행 수·canary 행을 대조합니다. 첫 실행에서 client가 입력을 가로채 테이블 1개만 검사하던 결함을 CI 로그로 찾아 고쳤습니다.
  - C-05 Windows 설치 파일(PR #63, `73067bd`): CI가 `School Collect_0.2.0_x64-setup.exe`(약 2.97MB)를 매번 만들고 서명 상태(`NotSigned`)를 기록합니다. updater는 켜지 않았습니다.
  - C-04 전체 이력 스캔(PR #64, `8829326`): 155개 커밋에서 HEAD 0건, 과거 revision의 rewrite 후보 3묶음을 [SECURITY_BASELINE.md](SECURITY_BASELINE.md)에 기록했습니다. 값은 출력하지 않습니다.
  - S-05 PKCE·state 검증(PR #65, `d6c86a3`): 단위 테스트 7개(RFC 7636 공식 값 포함). 로그인 화면 연결은 redirect 허용 목록 결정 후입니다.
  - C-01 Select·Checkbox(PR #66, `cbddb20`): 실제 브라우저에서 접근성 이름·키보드 토글·40/36px 높이·640px 넘침 없음을 확인했고, 검증 계정·학교를 지웠습니다(잔여 0).
  - 이 시점 `cargo test --workspace --locked`는 125개입니다.
- 학교 화면의 로그인 이후 흐름과 과거 native/API/DB·브라우저 검증, 외부 운영 설정은 이번에 재실행·재조회하지 않았습니다. CI 성공도 production 배포를 대신하지 않습니다.

## 학교업무 / 개인기능 상태

| 영역 | 통합된 것 | 통합 전 구현 | 남은 우선 작업 |
| --- | --- | --- | --- |
| 학교업무 | PR #19·#24(`f573bfb`) + S-02(`9416c6c`) + S-03(`70dcbfe`) + S-06(`e722bce`) + S-05 세션·초안(`93cebdc`)·브라우저 로그인 흐름(`9579a05`) + S-07 내보내기·역할 검증(`9860710`, `88b0a38`, `9b63abf`) + S-04 첨부 metadata·API·sweep·화면·R2 adapter(`e3a541c`, `2486f8d`, `aaa55c5`, `64aad02`, `04de924`) | 없음 | S-05 로그인 화면 연결(provider redirect 등록 필요), S-04 실제 R2 bucket 연결(bucket·credential 필요) |
| 개인기능 | PR #25 개인 화면(`11a0af0`) + PR #30 브리지·확장(`795dc26`) + PR #32·#33 자동입력 엔진(`5318a96`) + PR #35 표 입력(`09b3f88`) + PR #37 감시(`e5415dc`) + PR #51 레시피 추천(`8704d95`) 통합 | 없음 | P-05 앱 데이터 연결(데이터 원천·권한 계약 대기) |
| 공통 기반 | PR #17 DS/AppShell, PR #20 Docker 스택, PR #24 병합에서 학교/개인 화면 경계 정리, PR #54 설정·narrow 보완 | 없음 | C-01 Figma DS 확장, C-02~C-05 보안·운영·배포(소유자 결정·외부 자원 필요) |

PR #24/#25는 모두 develop에 병합됐습니다(`f573bfb`, `11a0af0`). 병합 과정에서 학교 화면 구조는 PR #24 방식을 유지하고 개인기능은 `pages/AutomationPage.tsx`와 `#automation` 라우트로 분리했습니다.

## 독립 검토 (2026-09-27)

계획 문서의 완료 상태를 외부 모델(gpt-6-astra, reasoning 매우높음)에 read-only 적대적 검토로 요청했습니다. 판정은 **BLOCK**(승격·실제 배포 준비 불가)이었고, 지적과 처리 결과는 다음과 같습니다.

- 수정: 초안 native 명령이 렌더러가 보낸 사용자 id를 그대로 썼습니다. 이제 저장된 세션의 사용자와 다르면 거부하고(`session_user_matches`), 로그아웃은 초안 정리를 기다린 뒤 세션을 지웁니다. 세션·초안 저장/삭제 실패는 화면 배너로 알립니다.
- 수정: E2E 정리가 panic 경로에서 보장되지 않았습니다. `TestCleanup` 가드가 panic에도 provider 계정과 행을 지우고, 네 테스트 모두 정리 후 남은 행 0을 확인합니다.
- 수정: 확장 권한 문서가 `scripting`을 빠뜨렸고, 감시의 "본문 미저장" 문구가 사용자 선택을 반영하지 못했습니다. 문서와 확장 README를 실제 동작에 맞췄습니다.
- 검증 보강: 통합 SHA `02f16a2`와 수정 통합 SHA `67e7dc1`에서 실제 provider·PostgreSQL E2E 4개를 다시 통과시켰고, 남은 계정·행이 0임을 확인했습니다. `394f9c9 -> 02f16a2` 차이는 문서뿐이라 그 이전 CI(V2 Architecture 성공)가 같은 앱·서버 코드를 덮고, `02f16a2 -> 67e7dc1`은 이 검토 대응 변경(초안 접근·정리·문서)입니다.
- 남은 blocker(소유자 결정·외부 자원 필요): S-04 실제 R2 bucket·credential, S-05 provider redirect 등록, C-03 staging/prod·최소 권한 DB·운영 DB 복구 훈련, C-05 서명·업데이트. 승격 전에 닫아야 합니다(1차 검토 시점 기록, 이후 진행은 아래 2차 검토 절).
- 검토자가 저장소 안에서 선행 가능하다고 밝힌 범위를 모두 진행했습니다: S-04 attachment contract·storage port·개발 adapter·만료 sweep(PR #58~#60), C-03 backup/restore drill(#62), C-05 설치 파일 번들(#63), C-04 full-history scan과 rewrite 범위(#64), S-05 PKCE state 검증(#65), C-01 Select·Checkbox(#66). Dialog·Sheet·Toast와 updater는 각각 Figma 승인과 서명 키가 필요해 남겨 두었습니다.
- 자동화 화면 확인: 1차 검토의 MEDIUM 지적(origin만 비교)에 따라 origin과 path를 함께 비교하도록 고쳤습니다. 화면 안의 월·학기 식별은 아직 없습니다([AUTOMATION_DESIGN.md](AUTOMATION_DESIGN.md) "화면 확인").

### 2차 독립 검토 (2026-09-27, HEAD `033fa00`)

같은 검토자(gpt-6-astra, reasoning 매우높음)에게 1차 지적이 닫혔는지 다시 확인받았습니다. 판정은 **BLOCK**이었고, 저장소 안에서 고칠 수 있는 지적은 모두 다음 PR에서 고쳤습니다.

- 초안 소유자(F1): 세션의 사용자 id를 token의 `sub`와 같을 때만 저장하고, 초안 명령도 저장된 token의 `sub`와 대조합니다. 앱은 앱 DB id 대신 provider subject로 초안을 엽니다. 실제 Tauri 창에서 로그인 → API 중단 중 임시 저장(로컬 SQLite, 소유자 = token subject) → 앱 재시작 후 로그인 유지·초안 복구 → 로그아웃 시 세션·초안 0행을 확인했고, 다른 사용자 id의 초안 조회와 subject가 다른 세션 저장은 거부됐습니다.
- 정리 실패 안내(F2): 세션·초안 삭제 래퍼가 오류를 삼키지 않고, 로그아웃은 두 단계의 실패를 모두 모아 안내합니다.
- 첨부(F3·F4·F7·F8): 저장된 첨부 재업로드 409, 삭제는 행 잠금 안에서 제출 여부를 확인한 뒤 행 → bytes 순서, 슬롯 열기는 배정 행 잠금, 삭제 표시된 행도 보존 기간 뒤 정리(migration `0006`).
- worker(F5·F6): 배치 사이에 주기를 온전히 기다리고(가상 시간 테스트), migration을 실행하지 않으며, 개발·배포 compose가 worker에 DB·NATS·첨부 경로를 전달하고 api와 첨부 volume을 공유합니다.
- E2E 정리(F9): provider 계정 삭제 실패를 오류로 모으고, 앱 사용자는 id와 provider subject로 함께 찾아 지웁니다.
- 이력 스캔(F10): 허용 목록은 비밀 값 전체가 placeholder와 같을 때만 통과하고, "HEAD에 있음"은 경로가 아니라 HEAD 내용으로 판정합니다(회귀 테스트 3개).
- PKCE(F11): 오류 응답도 state가 맞을 때만 받아들입니다.
- 자동화 화면(F12): PR #68로 origin+path 비교를 통합했습니다.
- 검토가 지적한 문서 부정확: #64는 문서·스크립트만 바꿔 `repository-checks` 한 개만 실행됐습니다. "#62~#66 각각 CI 7개"는 #64에 대해 사실이 아니었습니다. `docs/PERSONAL_PLAN.md`의 확장 권한에 `scripting`을 추가했습니다.

2차 검토가 "저장소 안에서 더 할 수 있는 것"으로 분류한 나머지 작업도 통합했습니다. 각 PR은 병합 전 CI 7개가 성공했습니다.

- 첨부 화면(PR #70, `64aad02`): 제출 작성 화면의 항목별 첨부·받기·지우기, 관리자 수합 상세의 제출 첨부 받기. 실제 Tauri 창에서 올리기 → `.exe` 요청 전 거절 → 받기(원본과 바이트 일치) → 지우기(저장소 0개) → 첨부 후 제출 → 관리자 화면 표시를 확인했고, 640·1280px 가로 넘침이 없었습니다. 이 검증에서 API CORS가 `DELETE`를 막던 결함을 찾아 고치고 preflight 테스트로 고정했습니다.
- R2 adapter(PR #71, `04de924`): `R2Storage`가 같은 저장소 port를 SigV4로 구현하고, `R2_*` 네 값이 모두 있으면 api·worker가 R2를 씁니다(일부만 있으면 기동 거부). 서명은 AWS 공개 예제 값과 일치하고, 요청 흐름은 loopback S3 stand-in으로 확인했습니다(presigned PUT 포함).
- 브라우저 로그인 흐름(PR #72, `9579a05`): loopback listener → authorize URL → 콜백 1회 검증 → verifier로 code 교환, 취소·만료 시 listener 종료. 가짜 provider 테스트 5개(성공·재생 거부·다른 state·provider 오류·거절 code·취소·만료·http endpoint 거부).
- 설치 수명 주기(PR #73, `6508be1`): CI Windows runner에서 0.2.0 설치 → 0.2.1로 같은 위치 업그레이드 → 제거 후 파일·제거 항목 없음을 확인했습니다.
- 이 시점 `cargo test --workspace --locked`는 141개, foundation 34개입니다.

### 3차 독립 검토 (2026-09-27, `c526ac3`)

같은 검토자(gpt-6-astra, 매우높음)의 판정은 **REVISE_LOCAL**이었습니다. 2차 지적 F1·F2·F4~F8·F10~F12와 R2 mock·PKCE 흐름은 닫힘으로, F3·F9·첨부 UI·설치 smoke는 부분으로 판단했고, 남은 저장소 안 결함을 PR #75(`60f2bcb`, CI 7개 성공)에서 모두 고쳤습니다.

- 동시 올리기: 슬롯을 한 번의 UPDATE로 차지한 요청만 bytes를 씁니다(migration `0007`). 실제 DB에서 8개 동시 차지 → 1개 성공·7개 거절을 확인했습니다.
- 실패한 올리기: 어느 단계에서 실패해도 bytes를 지우고 슬롯을 돌려줍니다. 앱은 빈 슬롯을 지우거나, 남으면 목록에서 지울 수 있게 보여 줍니다.
- 관리자의 자기 제출 화면은 `scope=mine`으로 자기 파일만 받습니다.
- 다운로드 저장은 `create_new`로 덮어쓰기 없이 한 번에 만듭니다(16개 병렬 저장 → 16개 파일).
- E2E는 provider 계정을 만든 직후 정리 대상으로 등록합니다.
- 설치 smoke는 제거 후 설치 폴더 전체가 사라졌는지 확인합니다(CI: `removed the install directory and uninstall entry`).
- `R2_ENDPOINT`는 `scheme://host[:port]`만 받습니다.
- 이 시점 `cargo test --workspace --locked` 144개, 실제 provider E2E 5개, 첨부 DB 테스트 7개, 백업 drill은 migration 7개·테이블 13개입니다.

## 단계 상태

| 단계 | 상태 | 확인된 내용 | 남은 게이트 |
| --- | --- | --- | --- |
| 1. 보안/기준점 | 추적 파일 정리 완료 | public 저장소 확인, PR #10 main merge, PR #12 동기화, v1 애플리케이션·업무자료 추적 제거, Infisical dev 서비스 토큰 회전(2026-09-25) | history rewrite 범위/실행, 나머지 외부 credential 정리 |
| 2. 협업 | 보호 적용 완료 | develop/작업 브랜치/문서/CI 구성, main·develop branch protection 적용 및 API 확인 | 협업자 추가 시 approval >= 1 및 code owner review, 독립 리뷰 |
| 3. 실행 기반 | 조건 충족 | lockfile 커밋 + frozen/locked CI, TS strict, fmt/clippy/test, PostgreSQL 18 migration, /health·/ready 분리 검증, Windows native build, local tauri dev | mobile/Linux/macOS 미검증, client production CSP·환경 분리 후속 |
| 4. Figma | 시안 승인 | Design System/Product 핵심 UI 승인, Stage 5 기본 token/component mapping | interaction/accessibility detail, Library/Code Connect 후속 |
| 5. Code DS/AppShell | 완료 (PR #17, `9697363`) | `packages/ui` token/component와 실제 Tauri AppShell, 상태·키보드·wide/narrow 검증 | Figma Library/Code Connect 후속 |
| 6. Auth/Data/Ops | 구현 중 | Supabase Auth 실로그인 + ES256/JWKS 검증, user/membership provisioning, tenant RBAC, Collect 상태 전이와 version 충돌까지 실제 프로젝트·실제 DB에서 E2E 통과. C-02로 키 회전·장애·캐시, 네 역할, 요청 ID·redaction, `/metrics`와 풀 재사용 학교 격리를 보강. S-06 outbox relay·S-05 세션/초안·S-04 첨부 metadata·API·sweep을 통합 | R2 실제 연결, backup/restore drill, 환경 분리 (NATS relay·SQLite 초안·세션 영속화는 통합) |
| 7. Collect | 구현 중 | PR #19 기본 흐름, PR #24 항목·대상·제출 현황·내 배정·구성원 화면, S-03 편집, S-07 결과 내보내기·화면·역할 4종 검증, S-04 첨부(metadata·API·sweep·화면·R2 adapter)를 통합하고 실제 프로젝트 E2E 5개와 실제 창으로 확인 | 실제 R2 bucket 연결, offline 세부 |

## 확인된 원격 상태

### Public repository / security
- repository visibility: public
- public 유지가 사용자 의도임
- 과거 `.env.example` 이력에서 실제처럼 보이는 외부 서비스/DB credential 형태 값 확인
- 과거 local CLI state에 project connection metadata 추적 이력 확인
- `_agent_작업` 아래 실제 업무에서 생성된 source/derived artifact 존재 확인
- 현재 HEAD 노출 축소용 hotfix PR #10이 main에 merge됨 (merge commit `3a7acd9`)
- PR #12로 main -> develop 동기화 완료 (merge commit)
- 이후 소유자 결정으로 v1 웹 애플리케이션과 `_agent_작업` 업무자료를 추적 대상에서 전부 제거. 현재 HEAD에는 실제 업무자료가 없음
- 추적 제거는 과거 Git object를 삭제하지 않으므로 history 정리는 여전히 별도 작업

credential 값 자체는 문서에 기록하지 않습니다.

### 승격 이력

- `develop` 통합: PR #19(실제 Supabase 인증·DB 연결과 첫 Collect 수직 슬라이스), PR #20(개발자별 Docker 개발 스택과 로컬 실행 절차)을 squash merge했습니다.
- `develop -> main` 승격: PR #21. 통합 후보 SHA는 `3ac8d1f`이고 merge commit은 `49f856a`입니다. 승격 후 main CI(Foundation) success를 확인했습니다.
- `main -> develop` 동기화: PR #22. merge commit은 `b50ff87`이고, 승격 merge commit이 develop에 포함됩니다. 두 브랜치의 트리는 동일합니다.
- 내용 변경 없는 승격·동기화이므로 이 시점의 배포나 production migration은 없습니다.

### Git protection
classic branch protection을 `main`과 `develop`에 적용하고 API 조회로 확인했습니다. pull request 필수, stale approval dismissal, conversation resolution, required check `repository-checks`, force push/삭제 차단, enforce_admins가 두 브랜치 모두 활성입니다. `main`만 strict(최신 base 강제)를 적용했습니다.

approval 최소 개수는 0입니다. 단독 운영 상태에서 1 이상이면 본인 PR을 병합할 수 없기 때문이며, 협업자 추가 시 함께 올려야 합니다. 자세한 내용은 [BRANCH_PROTECTION.md](BRANCH_PROTECTION.md)를 봅니다.

### Vercel 연동
v2 전환 후 남아 있던 Vercel `school-collect` 프로젝트와 배포를 2026-09-23에 삭제했습니다. 이 저장소는 Vercel 배포를 사용하지 않습니다.

### Stage 3
PR #11이 develop에 squash merge되었습니다 (`4cadf7e`).

Stage 3 완료 항목:
- `pnpm-lock.yaml`(root + `apps/app` importer)과 `Cargo.lock`을 커밋해 source of truth로 확정
- CI의 모든 install을 `pnpm install --frozen-lockfile` / cargo `--locked`로 전환
- `--exclude school-collect-app` 제거. clippy `-D warnings`와 test가 Tauri crate 포함
- 실제 개발 머신에서 `tauri dev` 확인. "School Collect" 창 표시, Vite dev server 200 응답
- 실제 개발 머신에서 `tauri build --no-bundle` 성공
- PostgreSQL 18 빈 DB 확인 → v2 migration → 결과 검증 → 재실행 idempotency를 CI에서 검증
- production migration에 demo seed table이 생성되지 않음을 CI 검사로 보장
- `/health`와 `/ready` 의미 분리를 실제로 검증. `DATABASE_URL` 누락은 기동 실패, 도달 불가 DB는 `/ready` 503, 도달 가능 시 200

Stage 3 미검증 항목:
- mobile target/plugin compatibility: Android SDK/NDK 부재, Rust target 미설치, iOS는 macOS 필요. CLI에 `android init`/`ios init`가 존재하는 것만 확인
- dev/prod config 분리: `tauri.conf.json` CSP `connect-src`가 아직 `http://127.0.0.1:*` 허용. production 교체는 Stage 6
- Linux/macOS native build 미검증

### Stage 4 Figma
사용자가 현재 Figma 시안을 승인함.

승인 핵심:
- light compact sidebar
- vertical active indicator 제거
- selective Card
- content/action density 기반 card sizing
- peer collection의 단독 Card 강조 금지
- List Surface + uniform List Row
- semantic state로 priority/urgency 표현
- radius <= 8px
- decorative shadow 없음

## Stage 5 Code Design System / AppShell

첫 번째 code-only migration은 PR #17에서 완료되었습니다.

- `packages/ui`: semantic token, Button, Status, Card, List Surface/Row, Tabs,
  DataTable, FormField, Sidebar, Header, AppShell, Empty/Loading/Error/
  Permission/Offline state primitive
- `apps/app`: 승인된 compact shell, hash 기반 navigation/history, 실제 `/health`
  확인, API error와 browser offline recovery state
- 검증: typecheck, frozen install, build, browser 1280/1024/720 viewport,
  keyboard focus, offline event
- 이 배치는 Figma canonical file을 쓰지 않으며, 실제 업무 데이터나 demo seed를
  추가하지 않습니다. 상세 mapping과 rollback은
  [STAGE5_CODE_MIGRATION.md](STAGE5_CODE_MIGRATION.md)에 기록합니다.

기본 Stage 5는 PR #17로 통합됐습니다. 이후 C-01에서 새 학교업무/개인기능의 진입 경계와 Figma interaction/accessibility·Library/Code Connect를 보강합니다. 과거 완료된 기본 구현을 다시 미착수로 표시하지 않습니다.

## Production-first 데이터 정책

- production migration에 demo school/user/task/submission seed를 넣지 않음
- 저장소에 가상 fixture DB나 실제처럼 채운 demo dataset을 상시 보존하지 않음
- 테스트 실행 시 필요한 데이터만 생성하고 rollback/truncate/disposable DB 등으로 제거
- 실제 업무자료/개인정보는 public repository에 두지 않음
- 제품에 필수인 reference data는 demo data와 구분

## 실제 연결된 개발 환경

- 개발 환경: 개발자별 Docker 스택(`infra/compose.dev.yml`)입니다. 공용 개발 서버와 공용 개발 DB를 두지 않습니다.
- identity/database: 등록된 Supabase 프로젝트(Andong ICT Infisical `school collect`, Development)
- v2 객체는 `school_collect` 스키마에 있고, v1 잔여 테이블(`public`)과 분리되어 있습니다.
- 검증: `services/api/tests/collect_flow_e2e.rs`가 실제 로그인 → 토큰 검증 → 수합 생성·배포·작성·제출·마감과 tenant 격리를 확인하고 생성한 행과 테스트 계정을 모두 삭제합니다.
- 위험: dev/staging/prod Infisical 라벨이 **같은** Supabase 프로젝트를 가리킵니다. 환경 분리는 아직 없습니다.
- 참고: staging용 Supabase 프로젝트 생성은 무료 한도(계정당 무료 프로젝트 2개)에 걸려 있어 소유자 결정이 필요합니다.

## 데스크톱 클라이언트 (Stage 5 이후 첫 제품 기능)

- `apps/app`은 로그인 → 학교 등록 → 수합 생성 → 배포 → 초안 저장 → 제출을 실제 API와 연결합니다.
- 세션(접근·갱신 토큰)은 이 컴퓨터의 OS 자격 증명 저장소에 보관해 앱을 다시 열어도 로그인이 유지되고, 로그아웃하면 저장된 세션을 지웁니다. 비밀번호는 저장하지 않습니다.
- 검증: 로컬 API(`APP_AUTH_MODE=oidc`)와 dev 서버를 띄운 상태에서 실제 브라우저로 전체 흐름을 확인했습니다. 검증용 계정과 데이터는 삭제했습니다.

## 제품 틀과 통합 기록

### 학교업무 제품 틀 (S-01)

역할·객체·화면·API 표면은 [MVP_SCOPE.md](MVP_SCOPE.md)에 고정했습니다. PR #24(`feat/collect-mvp-frame`)로 develop에 통합했습니다.

- 화면: 홈, 자료수합(+상세), 내 제출(+제출 작성), 구성원, 설정. 해시 라우팅과 권한별 메뉴 노출을 포함합니다.
- 서버: 수합 생성 시 항목(`collect_items`)과 대상(`collect_assignments`)을 한 transaction으로 기록하고, 배포가 상태를 `published`로 바꾸며, 제출 현황(`/v1/collects/{id}/status`)과 내 배정(`/v1/assignments`), 구성원(`/v1/members`)을 제공합니다.
- 검증: `services/api/tests/collect_flow_e2e.rs`가 실제 Supabase 로그인과 실제 PostgreSQL에서 항목 저장, 기본 대상 생성, 제출 현황 수치, 내 배정 목록, 잘못된 항목 key 거부를 함께 확인합니다. 로컬 Docker 스택에서는 브라우저로 홈 → 수합 생성(항목 2개) → 배포 → 제출 작성(임시 저장 버전 1, 제출) → 관리자 현황 `1/1`, 미제출 없음까지 확인했습니다.
- 남은 위험: 구성원 초대 경로가 아직 없어서 관리자가 교사를 자기 학교에 추가할 수 없습니다. 교사 화면은 membership이 있는 사용자에게만 열리므로, 초대 기능(S-02)이 다음 슬라이스입니다.

### 개인 바로가기 (P-01)

- PR #25(`feat/work-automation-shortcuts`)로 develop에 통합했습니다(`11a0af0`).
- 로그인·학교 membership 없이 `업무 자동화` 화면을 열고, 렌더러는 `open_automation_recipe(recipeId)`만 호출합니다. 열 주소는 native가 저장된 레시피에서 다시 읽어 검증합니다.
- `url` crate 검증, 브라우저 열기 분리 실행, 레시피 100개·격리 512 KiB 상한, Unix 0600 권한, 삭제 확인·실행 취소·`aria-label`을 포함합니다.
- 실제 데스크톱 창에서 로그인 없이 화면 열기 → 등록(92바이트 기록) → 열기 → 삭제(0바이트) → 실행 취소(92바이트 복원) → 앱 재시작 후 보존까지 확인했고, 검증용 레시피는 삭제했습니다. 상세는 [AUTOMATION_DESIGN.md](AUTOMATION_DESIGN.md)에 있습니다.
- 남은 범위: 브라우저 확장·현재 화면 등록(P-02), DOM 입력(P-03 이상). 레시피는 아직 OS 사용자 단위입니다.

### 브라우저 확장 브리지 (P-02)

- PR #30(`feat/personal-bridge-extension`)으로 데스크톱 앱의 로컬 브리지와 `apps/extension` Chrome/Edge 확장을 develop에 통합했습니다(`795dc26`).
- 브리지는 `127.0.0.1:43110`에만 열리고 토큰(`automation-bridge.token`)과 고정 확장 ID origin(`dfobjphjganlegjomdgmaaphbjcgpoea`)을 함께 확인합니다. 제공하는 것은 상태 확인과 바로가기 생성뿐이며 URL 열기·DOM 접근·임의 JavaScript 실행은 없습니다.
- 확장 권한은 `activeTab`, `storage`, `http://127.0.0.1:43110/*`뿐이고, 등록 값은 현재 탭의 제목과 주소입니다. `scripts/tests/test_extension_contract.py`가 공개 키에서 확장 ID를 다시 계산해 브리지 상수와 대조합니다.
- 검증: Rust fmt/clippy/테스트(워크스페이스 46개), 앱 typecheck/build, foundation 24개, repository guard, 실제 소켓 스모크(200/201/400/401/403), 실제 Tauri 창에서 연결 카드와 브리지 생성 버튼 표시.
- 검증(실제 브라우저, Edge 154): 임시 프로필에 확장을 로드해 실제 ID가 `dfobjphjganlegjomdgmaaphbjcgpoea`로 열리는 것, 토큰 저장·연결 확인·현재 화면 등록으로 레시피가 기록되는 것을 확인하고 검증 레시피는 되돌렸습니다.
- 아직 아님: 도구 모음 아이콘 클릭 팝업과 `activeTab` 부여 경로, 실제 업무 사이트 주소 등록. 상세는 [AUTOMATION_DESIGN.md](AUTOMATION_DESIGN.md)의 2단계 절에 있습니다.

### 자동입력 레시피 (P-03)

- 1차(PR #32, `455dcdc`): 자동입력 레시피 저장 형식(`kind=fill`, 화면 주소 + 필드 목록), 확장의 요소 선택·위치 후보 생성, dry-run 미리보기.
- 2차(PR #33, `5318a96`): 값 입력 실행과 결과 검증, 모호한 위치·쓸 수 없는 컨트롤에서 아무것도 입력하지 않고 중단. 저장·제출 버튼은 누르지 않습니다.
- 위치는 `id`/`name`/`label`/`css`만 허용하고, CSS는 문자 집합과 함수형 선택자를 제한합니다. 브리지에 `POST /v1/bridge/fill-recipes`, `GET /v1/bridge/recipes`를 추가했고 토큰·고정 origin 검사는 그대로입니다.
- 검증: Rust fmt/clippy/테스트(워크스페이스 55개), 앱 typecheck/build, foundation 24개, repository guard, 실제 브라우저(Edge)에서 요소 선택·저장·미리보기·실행·중단 경로, 실제 Tauri 창에서 목록 표시와 두 단계 삭제.
- 아직 아님: 학생 × 날짜 행렬, 감시, 실제 업무 사이트에서의 사람 검증. 상세는 [AUTOMATION_DESIGN.md](AUTOMATION_DESIGN.md)의 3단계 절에 있습니다.

## 배포 기반

- `infra/api.Dockerfile`과 `infra/compose.deploy.yml`로 서버 이미지·구성을 정의했습니다.
- 실행 순서와 소유자 결정이 필요한 항목은 [DEPLOYMENT.md](DEPLOYMENT.md)에 정리했습니다.
- 호스팅 위치, 도메인·TLS, DB 역할 분리, 데스크톱 서명은 아직 결정되지 않았습니다.

## 아직 하지 않은 것

- Git history rewrite
- 나머지 외부 credential 실제 폐기/회전 확인 (Infisical dev 서비스 토큰은 2026-09-25 회전 기록 있음)
- production infra 생성/배포
- R2 실제 저장소 연결과 production 연결, 학교 환경 분리
- production DB migration

인증 provider는 [ADR-0002](ADR/0002-supabase-identity-and-database.md)의 Supabase Auth이며 ZITADEL 구축은 현행 대기 작업이 아닙니다. 클라이언트 세션은 OS 자격 증명 저장소에 저장되어 재시작 후에도 유지됩니다(S-05). 외부 브라우저 PKCE 로그인은 identity provider의 redirect 허용 목록 결정이 필요해 후속으로 남아 있습니다.
