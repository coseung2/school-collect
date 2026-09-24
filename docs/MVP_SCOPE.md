# 제품 틀 (첫 배포 가능 범위)

기준일: 2026-09-25. 이 문서는 [V2_PLAN.md](V2_PLAN.md) Stage 7(Collect reference
implementation)로 가는 **첫 제품 틀**을 고정합니다. 구현된 것과 아직 아닌 것을
구분해서 적고, 팀이 이 틀 위에서 기능 브랜치를 딸 수 있게 하는 것이 목적입니다.

## 이 틀에 포함되는 것

역할, 객체, 화면, API 표면을 지금 확정합니다.

### 역할

| 역할 | 할 수 있는 일 |
| --- | --- |
| `admin` | 수합 생성·배포·마감, 구성원 보기, 제출 |
| `coordinator` | `admin`과 동일 |
| `contributor` | 배정된 수합 작성·제출 |
| `viewer` | 열람 |

권한 판단은 항상 서버가 합니다. 클라이언트가 보낸 tenant/role은 검증 대상입니다.

### 객체

- **수합(Collect)**: 제목, 설명, 마감일, 상태(`draft -> published -> closed`).
- **수합 항목(CollectItem)**: `key`, 표시 이름, 필수 여부, 순서. 제출 payload의 키와
  일치합니다.
- **배정(Assignment)**: 이 수합을 제출해야 하는 구성원. 생성 시 정하고 기본값은
  `viewer`를 제외한 전체 구성원입니다.
- **제출(Submission)**: 구성원별 `draft -> submitted`. `version`으로 동시 수정 충돌을
  탐지하고, 제출 후에는 수정할 수 없습니다.

### 화면 구조

| 화면 | 경로 id | 대상 | 내용 |
| --- | --- | --- | --- |
| 홈 | `overview` | 전체 | 내가 제출할 수합, 관리 중인 수합, 마감 임박 요약 |
| 자료수합 | `collects` | admin·coordinator | 수합 목록(상태 필터), 생성, 상세에서 배포·마감 |
| 수합 상세 | `collects/:id` | admin·coordinator | 항목, 제출 현황(제출/미제출 명단), 배포·마감 |
| 내 제출 | `assignments` | 전체 | 배정된 수합 목록과 내 제출 상태 |
| 제출 작성 | `assignments/:id` | 전체 | 항목별 입력, 임시 저장, 제출 |
| 구성원 | `members` | admin·coordinator | 구성원과 역할 목록 |
| 설정 | `settings` | 전체 | 계정, 연결 상태 |

각 화면은 Default/Loading/Empty/Error/Permission 상태를 구분합니다.

### 흐름

관리자: 수합 만들기(항목 + 대상) -> 배포 -> 상세에서 제출 현황 확인 -> 마감.

교사: 내 제출에서 배정 확인 -> 항목별 작성 -> 임시 저장 -> 제출. 마감 뒤에는 읽기만
가능하고, 저장·제출은 서버가 거부합니다.

### API 표면

| method | 경로 | 용도 |
| --- | --- | --- |
| `GET` | `/v1/session` | 사용자와 소속 |
| `POST` | `/v1/tenants` | 학교 등록(생성자 `admin`) |
| `GET` | `/v1/members` | 구성원과 역할 |
| `GET` | `/v1/collects` | 수합 목록(내 제출 상태 포함) |
| `POST` | `/v1/collects` | 수합 생성(항목·대상 포함) |
| `GET` | `/v1/collects/{id}` | 상세(항목, 내 제출, 진행 수치) |
| `GET` | `/v1/collects/{id}/status` | 제출 현황 명단(관리자) |
| `POST` | `/v1/collects/{id}/publish` | 배포 |
| `POST` | `/v1/collects/{id}/close` | 마감 |
| `PUT` | `/v1/collects/{id}/submission` | 임시 저장(`expectedVersion`) |
| `POST` | `/v1/collects/{id}/submission/submit` | 제출 |
| `GET` | `/v1/assignments` | 내가 제출할 수합 |

오류는 항상 `ApiError { code, message, requestId }` 형태입니다. version 충돌은
`409 version_conflict`와 현재 version을 함께 돌려줍니다.

## 이 틀에 아직 없는 것

아래는 다음 기능 브랜치의 대상입니다. 지금은 화면이나 API가 없다는 것이 정상 상태입니다.

- 파일 첨부(R2 업로드/다운로드, 보존 정책)
- 항목 편집과 배포 후 항목 변경 규칙
- 결과 내보내기(CSV/문서)와 재처리
- 구성원 초대·역할 변경 절차
- outbox/worker 기반 알림과 재처리
- offline 초안(SQLite)과 재연결 병합
- 외부 브라우저 PKCE 로그인과 OS 보안 저장소
- 배정되지 않은 구성원의 제출 차단 규칙 세분화

## 검증 기준

- 서버: tenant A/B 격리, 역할별 차단, 상태 전이, version 충돌, 마감 후 차단이
  실제 PostgreSQL 테스트로 통과합니다.
- 클라이언트: typecheck/build가 통과하고, 로컬 서버에 연결해 관리자·교사 흐름을 실제로
  한 번씩 수행합니다.
- 테스트 데이터는 실행 중 생성하고 종료 시 제거합니다. 저장소에 demo seed를 두지
  않습니다.
