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
| S-04 | 파일 첨부 / 완료 1~3차(PR #58 `e3a541c`, PR #59 `2486f8d`, PR #60 `aaa55c5`) | S-01, C-03 승인된 저장소 설정 | 완료: metadata·보존 규칙, 업로드·다운로드 API와 저장소 port(학교 간 접근 차단), 만료 정리 worker / 남음(소유자 자원): private R2 adapter·presigned capability, 제출 작성·검토 화면 |
| S-05 | 세션 유지·offline 초안 / 진행 중(PR #45, PR #46, PR #65) | S-01, C-01/C-02 | 완료: OS 보안 저장소 세션(재시작 복구·갱신·로그아웃 정리)과 사용자/학교별 SQLite 초안(재시작 복구·재전송·충돌·로그아웃 정리), 외부 브라우저 로그인의 PKCE·state·loopback 콜백 검증 모듈(단위 테스트 7개) / 남음: provider redirect 허용 목록 등록(소유자) 후 로그인 화면 연결 |
| S-06 | 알림·재처리용 outbox/worker / 완료(PR #43, `e722bce`) | S-01 | 업무 변경+outbox 단일 transaction, relay/JetStream/idempotent 처리, commit 후 ACK, retry/dead-letter·중복 전달 복구 |
| S-07 | 결과 내보내기·전체 흐름 검증 / 완료(PR #48, PR #50, PR #53) | S-02~S-06, 출시 시 C-02~C-05 | 완료: CSV 결과의 권한·누락 검증, 화면 내보내기 버튼, 마감 후·다른 학교 내보내기 경계, 역할 4종 단일 시나리오를 통합 SHA에서 검증 |

공통 ID는 [TEAM_BACKLOG.md](TEAM_BACKLOG.md)를 따릅니다. 첨부·offline·worker는 S-01 계약이 고정된 뒤 독립 범위로 진행해 첨부는 metadata·API·정리 sweep까지 통합했습니다. 실제 운영 저장소(R2 계정·bucket) 연결과 제출 작성·검토 화면은 계정·환경이 확정된 뒤 진행합니다.

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

## outbox와 재처리 (S-06)

PR #43(`feat/outbox-relay`)로 업무 변경과 이벤트를 한 transaction에 쓰고 worker가 배달·재처리하는 경로를 develop에 통합했습니다.

- 이벤트 기록: `collect.created`, `collect.published`, `collect.closed`, `collect.submitted`, `membership.joined`를 각 업무 transaction 안에서 함께 씁니다. payload에는 식별자·상태·버전만 담고 학생 이름이나 본문은 넣지 않습니다.
- 선점과 lease: `claim_outbox_batch`가 `FOR UPDATE SKIP LOCKED`와 `claimed_until` lease로 배치를 예약합니다. 선점이 한 번의 commit되는 UPDATE라서 lease 없이는 두 relay가 같은 행을 함께 가져갈 수 있었고, 그 문제를 lease로 막았습니다. 죽은 worker의 행은 lease가 만료되면 다시 선점됩니다.
- 배달·재처리: 브로커 ack 뒤에만 `published_at`을 기록하고, 실패는 시도 횟수에 비례한 지연으로 재시도하며 한도를 넘으면 `dead_lettered_at`·`last_error`로 남깁니다. `/metrics`에 pending·published·dead-lettered gauge를 노출합니다.
- JetStream: 이벤트 id를 `Nats-Msg-Id`로 넣어 브로커가 중복 publish를 버리게 하고, 소비자는 event id로 중복을 제거합니다(at-least-once). `DATABASE_URL`·`NATS_URL`이 없으면 worker는 기동에 실패합니다.

검증 (2026-09-27):

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`(91개), foundation 25개, repository guard.
- 실제 개발 DB에서 relay 테스트 6개(배달 후 재선점 없음, 실패 → 재시도 → dead-letter, 동시 relay 중복 선점 없음, 업무 transaction과 이벤트 동시 기록·거부된 전이는 이벤트 없음, 선점 시 시도 횟수 증가, 재시도 예약) 통과.
- CI `postgres` 작업이 DB 쪽 relay 테스트를, 새 `nats-outbox` 작업이 NATS JetStream 컨테이너와 함께 실제 배달·ack·중복 제거(같은 id 두 번 publish → 1건)를 확인했습니다.
- 아직 아님: 이벤트를 소비하는 서비스(알림 발송 등). 이 슬라이스는 배달·재처리까지입니다.

## 결과 내보내기 (S-07 1차)

PR #48(`feat/result-export`)로 수합 결과 CSV 내보내기를 develop에 통합했습니다.

- `GET /v1/collects/{id}/export`(Manage 권한)가 항목 머리글과 `이름·역할·배정 상태·제출 상태·제출 시각`, 항목별 값을 CSV로 돌려줍니다.
- 배정된 구성원은 제출하지 않았어도 행이 나오고 항목 칸이 비어 있어 누락이 파일에서 드러납니다. 값은 쉼표·따옴표·줄바꿈이 있을 때만 인용하고 내부 따옴표를 두 번 반복하며, Excel용 UTF-8 BOM을 붙입니다.
- contributor·viewer는 403이고, 내보내기는 `collect.exported` 감사 이벤트로 남습니다.

검증 (2026-09-27): `cargo fmt/clippy/test`(98개), foundation 25개, repository guard, CSV 인용·누락 단위 테스트 2개, 실제 프로젝트 E2E `result_export_end_to_end`(담당자 200·두 구성원 행·쉼표 값 인용·감사 1건, 교사 403). 이 시점의 남은 것은 통합 시나리오와 화면 버튼이었고, 아래 2차·3차에서 채웠습니다.

## 결과 내보내기 화면과 전체 흐름 검증 (S-07 2차)

PR #50(`feat/export-ui`)으로 화면 내보내기와 전체 흐름 검증을 develop에 통합했습니다.

- 수합 상세(Manage 권한)에 `결과 CSV 내보내기` 버튼을 추가했습니다. API에서 CSV를 받아 native 명령으로 다운로드 폴더에 저장하고 안내 줄에 저장 경로를 보여 줍니다.
- 파일 이름은 수합 제목과 날짜로 만들고, native가 경로 문자·상위 이동·숨김 이름·줄바꿈을 거부합니다. 다운로드 폴더 밖에는 쓰지 않고 같은 이름이 있으면 `(1)`을 붙이며 내용은 4MiB로 제한합니다.
- 전체 흐름 E2E에 마감된 수합의 내보내기(제출 값이 CSV에 남는지)와 다른 학교 관리자의 내보내기 403을 추가했습니다. 제출 payload를 실제 항목 키(`plan`)로 바꿔 내보내기 열 매핑까지 검증합니다.

검증 (2026-09-27): `cargo fmt/clippy/test`(100개), 앱 typecheck/build, foundation 25개, repository guard, 실제 Tauri 창에서 내보내기 저장·안내 표시와 파일 내용(UTF-8 BOM + 항목 머리글 + 제출 값), 실제 프로젝트 E2E 3개(전체 흐름·초대·내보내기). 검증용 계정·행·CSV 파일은 정리했습니다.

## 역할 4종 통합 검증 (S-07 3차)

PR #53(`feat/role-sweep-e2e`)으로 한 학교·한 시나리오에서 네 역할을 도는 E2E를 develop에 통합했습니다.

- 역할마다 별도 계정을 만들고, admin이 학교를 만든 뒤 coordinator·contributor·viewer를 역할을 지정해 초대하고 각자 수락합니다. `GET /v1/members`에 네 역할이 모두 나와야 합니다.
- admin이 수합을 발행한 뒤: viewer는 목록·상세·배정 조회 200, 제출 저장·내보내기·진행 현황·항목 편집 403. contributor는 저장·제출 200, 내보내기·항목 편집 403. coordinator는 항목 편집 200, 진행 현황 200, 내보내기 200. admin도 같은 현황·내보내기를 확인합니다.
- viewer는 담당자가 될 수 없습니다(`target_not_assignable` 409). 그래서 내보내기 파일은 머리글 + 3행이고 viewer 이름은 나오지 않습니다.
- 테스트는 정리 후 남은 행이 0인지 스스로 확인합니다.

검증 (2026-09-27, Windows): `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`(101개). 실제 provider + 실제 PostgreSQL에서 통합 SHA `9b63abf` 기준으로 `collect_flow_e2e` 4개(전체 흐름·초대·내보내기·역할 4종)를 함께 통과시켰고, 남은 행 0과 provider 사용자 목록에 `@example.test` 계정이 없음을 확인했습니다.

## 파일 첨부 (S-04 1~3차)

PR #58(`e3a541c`)·PR #59(`2486f8d`)·PR #60(`aaa55c5`)으로 첨부 metadata 계약, 업로드·다운로드 API와 저장소 port, 만료 정리 worker를 develop에 통합했습니다. private R2 adapter와 제출 작성·검토 화면은 소유자 자원이 확정된 뒤 4차로 진행합니다.

- 계약·정책(domain): 파일 1개 최대 10 MiB, 항목당 5개, 제출당 20개입니다. 허용 형식은 HWP/HWPX·Office·PDF·PNG/JPEG·text·zip이고 보존은 180일입니다. 파일 이름은 표시용으로만 쓰고 경로 문자·과도한 길이를 거부합니다.
- metadata(DB): `school_collect.collect_attachments`가 (tenant, collect, 담당자, item key) 단위로 이름·형식·선언 크기·checksum·object key·상태(`pending`/`stored`/`deleted`)·만료 시각을 보관합니다. object key는 서버가 `tenants/{tenant}/attachments/{attachment_id}`로 만들고, 목록·조회·삭제·만료 수집 모두 tenant로 한정합니다.
- API·저장소 port: `POST /v1/collects/{id}/attachments`(슬롯 열기) → `PUT /v1/attachments/{id}/content`(bytes 업로드, 선언 크기·형식·checksum 검증) → `GET /v1/attachments/{id}`·`/content`, `GET /v1/collects/{id}/attachments`, `DELETE /v1/attachments/{id}`. 읽기는 소유자 또는 관리자, 삭제는 관리자 또는 제출 전 소유자입니다. bytes는 `crates/application`의 `ObjectStorage` port 뒤에 있고 개발은 `APP_ATTACHMENT_DIR`의 디렉터리 adapter를 씁니다. 저장소가 설정되지 않으면 503(`attachment_storage_unavailable`)으로 거절하고, 개발 외 환경은 `APP_ATTACHMENT_DIR` 없이 기동하지 않습니다.
- 보존 정리(worker): 한 배치를 끝낸 뒤 `WORKER_ATTACHMENT_SWEEP_MS`(기본 6시간)를 기다리고 다음 배치를 시작합니다. 삭제 표시된 행도 보존 기간이 지나면 지웁니다. bytes를 먼저 지우고 성공한 행만 삭제하며, 실패한 행은 남겨 다음 주기에 다시 시도합니다.
- 2차 검토 보강: 저장된 첨부의 재업로드 거절(409), 제출·삭제 경쟁과 동시 슬롯 열기를 행 잠금으로 막기, 행 먼저 숨기고 bytes 삭제. `crates/db/tests/attachments.rs`의 `concurrent_requests_cannot_break_attachment_rules`가 제한의 2배를 동시에 열어 정확히 5개만 만들어지는지와 제출 후 소유자 삭제 거절·관리자 삭제 허용을 실제 DB로 확인합니다.
- 검증 (2026-09-27, Windows): `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`(118개), foundation 30개, repository guard. 실제 개발 DB에서 `crates/db/tests/attachments.rs` 5개, worker sweep 1개, relay 6개가 통과했고, 실제 provider+PostgreSQL E2E 5개(전체 흐름·초대·내보내기·역할 4종·첨부)를 통합 SHA `aaa55c5` 기준으로 통과시켰습니다. 남은 `@example.test` 계정과 행은 0입니다.
- 아직: private R2 bucket·credential과 presigned 직접 업로드 capability, 제출 작성·검토 화면, 실제 네트워크 단절·재시도 사람 검증.

## 세션 유지와 offline 초안 (S-05)

PR #45(세션 보안 저장소)와 PR #46(오프라인 초안)로 세션 유지와 초안 보관을 develop에 통합했습니다. 외부 브라우저 PKCE 로그인은 남아 있습니다.

- 세션: 로그인하면 access token·refresh token·만료 시각·사용자 정보를 OS 자격 증명 저장소(Windows)에 저장합니다. 비밀번호는 저장하지 않습니다. 앱 시작 시 저장된 세션을 읽고, 만료 임박이면 refresh token으로 갱신하며, 실패하면 저장된 세션을 지우고 다시 로그인하게 합니다. 로그아웃은 저장된 세션을 지웁니다.
- 초안: 임시 저장·제출은 로컬 SQLite(`submission-drafts.sqlite`, 사용자·학교·수합 단위)에 먼저 쓰고 서버로 보냅니다. 서버가 받으면 로컬 초본을 지웁니다. 화면을 열 때 로컬 초안이 있으면 그 값으로 복구하고 안내를 보여 줍니다. 로그아웃은 그 사용자의 초안을 지웁니다.
- 한계: 항목 정의는 서버에서 오므로 작성 화면을 처음 여는 데는 연결이 필요합니다(초안 본문만 로컬). 외부 브라우저 PKCE는 identity provider의 redirect 허용 목록 결정이 필요해 별도 슬라이스로 둡니다.

검증 (2026-09-27, Windows):

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`(95개), 앱 typecheck/build, foundation 25개, repository guard. 초안 저장소 단위 테스트 3개(사용자·학교 분리, 재저장 교체, 로그아웃 정리, 잘못된 식별자·크기 초과 거부)를 포함합니다.
- 실제 Tauri 창: 로그인 → Windows 자격 증명 관리자에 세션 항목 생성 → 앱 재시작 후 로그인 유지 → 로그아웃 시 항목 삭제. API를 끈 상태에서 임시 저장 → 로컬 SQLite에 초안 기록, 재시작 후 제출 화면에서 값 복구, API 복구 후 임시 저장 → 서버 저장·로컬 초안 삭제, 로그아웃 → 재로그인 시 초안 없음을 확인했습니다. 검증용 계정·행·초안 파일은 정리했습니다.

인증과 membership RBAC를 새로 시작하는 작업으로 되돌리지 않습니다. `crates/auth`의 OIDC/JWKS 검증, `crates/db/src/records.rs`의 사용자 upsert·membership 조회, `services/api/src/lib.rs`의 권한 및 request ID 처리가 이미 있습니다.

C-02에서는 JWKS 갱신/장애, 잘못된 토큰, 네 역할(`admin`, `coordinator`, `contributor`, `viewer`), pooled connection의 학교 경계, 모든 오류의 request ID·redaction을 부정 테스트와 관측 근거로 보강합니다. RLS는 필요성을 판단하는 후속이며 적용 완료로 표시하지 않습니다.

## 이미 있는 기반과 남은 검증

## 변경 책임과 전체 완료 기준

업무 규칙은 `crates/domain`·`crates/application`, 저장·transaction은 `crates/db`·`migrations/v2`, 인증/API는 `crates/auth`·`crates/contracts`·`services/api`, 학교 화면과 초안은 `apps/app`, jobs는 `services/worker`를 기본 경계로 합니다. 공통 UI는 C-01과 조율합니다.

관리자 생성 -> 배포 -> 교사 합류/배정 -> draft -> 제출 -> 현황 -> 마감 -> 결과 내보내기를 연결합니다. 역할·학교 격리, 마감 후 차단, 중복 요청/idempotency, 동시 수정, 단절/재전송, 재시작 복구, export 재처리, 서버 기준 시간·상태 전이를 검증합니다.

native 앱, API, DB, CI, production 배포 여부를 각각 기록합니다. 개인 자동화 완성 여부는 학교업무 완료 조건에 포함하지 않습니다.
