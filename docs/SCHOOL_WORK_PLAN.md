# 학교업무 작업 계획

기준일: 2026-09-27 (Asia/Seoul). 기존 Stage 6/7, 이슈 #7/#8에 대응합니다.

학교 구성원이 수합을 만들고 배정받은 자료를 작성·제출하며 담당자가 현황과 결과를 확인하는 흐름입니다. 계정·membership·역할을 서버에서 확인하고 학교 데이터는 API를 통해 접근합니다.

## 범위와 현재 상태

- 학교 현황, 자료수합·상세, 내 제출·작성, 구성원, 학교 관련 설정이 이 영역입니다.
- `내 제출`과 offline 초안은 개인별이어도 학교 데이터입니다. 개인기능 저장소나 바로가기와 합치지 않습니다.
- `develop` `f573bfb`에 PR #19의 실로그인·사용자 연결·membership RBAC와 수합 기본 흐름, PR #24의 항목·대상·제출 현황·내 배정·구성원 화면이 통합돼 있습니다.
- PR #24는 CI 6개 성공 후 병합했고, 병합 과정에서 개인기능(`#automation`)과 화면 경계를 정리했습니다.
- 과거 API/DB·브라우저 검증 기록은 [STATUS.md](STATUS.md)에 있습니다. 이번 문서 정리에서 재실행하지 않았습니다.

## 작업 순서와 완료 조건

| ID | 작업 / 상태 | 선행 | 완료 조건 |
| --- | --- | --- | --- |
| S-01 | 항목·대상·제출 현황과 화면 확장 / 완료(PR #24, `f573bfb`) | C-01 공통 진입 계약 | develop 통합과 CI 6개 통과 확인. 다음은 S-02 |
| S-02 | 구성원 초대·합류 / 완료(PR #28, `9416c6c`) | S-01 | 초대된 계정이 해당 학교 contributor로 합류해 수합 제출; 다른 학교·계정에는 권한이 생기지 않음. 다음은 S-03 |
| S-03 | 항목·대상 편집과 권한 규칙 / 완료(PR #41, `70dcbfe`) | S-01, 구성원 변경 시 S-02 | 배포 전후 변경·역할 변경·미배정 제출 정책 확정, 서버 검증·충돌 처리·변경 기록과 UI 검증 |
| S-04 | 파일 첨부 / 계획 | S-01, C-03 승인된 저장소 설정 | 권한 확인 후 private R2 업로드/다운로드, 크기·유형·만료·보존 정책, 실패/재시도·학교 간 접근 차단 |
| S-05 | 세션 유지·offline 초안 / 계획 | S-01, C-01/C-02 | 외부 브라우저 PKCE + OS 보안 저장소, 사용자/학교별 SQLite 초안, 재시작 복구·재전송·충돌·로그아웃 정리 |
| S-06 | 알림·재처리용 outbox/worker / 계획 | S-01 | 업무 변경+outbox 단일 transaction, relay/JetStream/idempotent 처리, commit 후 ACK, retry/dead-letter·중복 전달 복구 |
| S-07 | 결과 내보내기·전체 흐름 검증 / 계획 | S-02~S-06, 출시 시 C-02~C-05 | CSV/문서 결과의 권한·누락·재처리 검증, 학교 A/B와 역할별 전체 흐름을 통합 SHA에서 검증 |

공통 ID는 [TEAM_BACKLOG.md](TEAM_BACKLOG.md)를 따릅니다. 첨부·offline·worker는 S-01 계약이 고정되면 독립 범위로 진행할 수 있습니다. 실제 운영 자원 연결은 계정·환경이 확정된 뒤 진행합니다.

## 구성원 초대 (S-02)

관리자가 이메일로 초대하고 교사가 로그인해 학교에 합류하는 경로를 구현해 develop에 통합했습니다(PR #28, `9416c6c`).

- 초대 코드는 서버가 만들고 SHA-256 해시로만 저장합니다. 평문 코드는 생성 응답에서 한 번만 돌려주며 관리자가 초대할 사람에게 직접 전달합니다(이 슬라이스에는 이메일 발송이 없습니다).
- 만료는 14일이고, 대기 중인 초대는 같은 학교·같은 주소에 하나만 존재합니다.
- 초대 역할은 contributor(기본)·coordinator·viewer로 제한합니다. admin은 초대로 부여할 수 없습니다.
- 이메일 발송 연동은 아직 없습니다. 관리자가 코드를 직접 전달하는 방식이며, 자동 발송은 별도 계정·권한이 필요한 후속 작업입니다.
- 수락은 토큰의 확인된 이메일이 초대 주소와 일치할 때만 성립하고, 학교와 역할은 초대 행에서만 결정합니다. 이미 구성원이면 기존 역할을 유지합니다.
- 구성원 화면에서 초대 생성·코드 1회 표시·취소·상태 목록을, 설정 화면에서 초대 코드 합류를 제공합니다.
- 검증: `services/api/tests/collect_flow_e2e.rs`의 `membership_invitation_end_to_end`가 실제 로그인과 실제 PostgreSQL에서 코드 해시 저장, 중복 초대 409, admin 초대 400, 다른 주소 수락 403, 합류 후 배정·제출·현황 집계, 코드 재사용 409, 취소 200/404, 만료 410, 미초대 계정 403을 확인하고 생성한 계정·행을 모두 삭제합니다.
## 항목·대상 편집 (S-03)

PR #41(`feat/collect-item-editing`)로 배포 전후 편집 규칙을 develop에 통합했습니다.

- 항목: 초안은 자유롭게 개편합니다. 배포 후에는 이름·필수 여부 변경과 항목 추가를 허용하고, 답변이 저장된 항목의 삭제는 거부합니다(`409 item_has_answers`). 마감된 수합은 편집할 수 없습니다(`409 collect_not_editable`).
- 대상: 제출 가능한 구성원만 대상이 될 수 있습니다(viewer·타 학교 사용자 거부 `409 target_not_assignable`). 초안을 저장했거나 제출한 대상의 제외는 거부합니다(`409 target_has_answers`).
- 미배정 제출: 대상이 아닌 구성원은 초안 저장·제출을 할 수 없습니다(`403 not_assigned`). 배포 시 대상 목록이 비어 있으면 제출 가능한 구성원 전체가 기본 대상이 됩니다.
- 충돌 처리: 두 편집 모두 `expectedVersion`을 요구하고 어긋나면 현재 버전과 함께 `409 version_conflict`를 돌려줍니다. 편집마다 버전이 오르고 감사 이벤트(`collect.items_updated`, `collect.assignments_updated`)가 남습니다.
- API: `PUT /v1/collects/{id}/items`, `PUT /v1/collects/{id}/assignments`(Manage 권한: admin·coordinator). 화면은 수합 상세의 `항목 편집`·`대상 편집`입니다.

검증 (2026-09-27, Windows):

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`(85개), 앱 typecheck/build, foundation 25개, repository guard.
- 실제 개발 DB: `crates/db/tests/collect_editing.rs` 2개가 항목 개편·배포 후 추가·답변 있는 항목 삭제 거부·버전 충돌·마감 후 거부와 대상 추가·viewer/외부인 거부·답변 있는 대상 제거 거부·미배정 저장·제출 거부를 확인했습니다. CI `postgres` 작업이 이 테스트를 실행합니다. 기존 `collect_flow_e2e` 2개도 그대로 통과했습니다.
- 실제 Tauri 창(WebView2 CDP): 로그인 → 자료수합 → 수합 상세 → `항목 편집`(이름 수정·항목 추가) 저장 → `대상 편집` 저장까지 화면에서 확인했습니다. 검증용 계정·학교·수합은 정리했습니다.
- 아직 아님: 거부 경로(답변 있는 항목 삭제 등)의 화면 조작 검증과 편집 이력 화면 표시.

## 이미 있는 기반과 남은 검증

인증과 membership RBAC를 새로 시작하는 작업으로 되돌리지 않습니다. `crates/auth`의 OIDC/JWKS 검증, `crates/db/src/records.rs`의 사용자 upsert·membership 조회, `services/api/src/lib.rs`의 권한 및 request ID 처리가 이미 있습니다.

C-02에서는 JWKS 갱신/장애, 잘못된 토큰, 네 역할(`admin`, `coordinator`, `contributor`, `viewer`), pooled connection의 학교 경계, 모든 오류의 request ID·redaction을 부정 테스트와 관측 근거로 보강합니다. RLS는 필요성을 판단하는 후속이며 적용 완료로 표시하지 않습니다.

## 변경 책임과 전체 완료 기준

업무 규칙은 `crates/domain`·`crates/application`, 저장·transaction은 `crates/db`·`migrations/v2`, 인증/API는 `crates/auth`·`crates/contracts`·`services/api`, 학교 화면과 초안은 `apps/app`, jobs는 `services/worker`를 기본 경계로 합니다. 공통 UI는 C-01과 조율합니다.

관리자 생성 -> 배포 -> 교사 합류/배정 -> draft -> 제출 -> 현황 -> 마감 -> 결과 내보내기를 연결합니다. 역할·학교 격리, 마감 후 차단, 중복 요청/idempotency, 동시 수정, 단절/재전송, 재시작 복구, export 재처리, 서버 기준 시간·상태 전이를 검증합니다.

native 앱, API, DB, CI, production 배포 여부를 각각 기록합니다. 개인 자동화 완성 여부는 학교업무 완료 조건에 포함하지 않습니다.
