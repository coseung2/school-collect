# School Collect v2

[한국어](#한국어) · [English](#english)

## 한국어

School Collect는 학교의 반복 행정업무를 수합·검증·자동화하기 위한 **오픈소스 업무 플랫폼**입니다. v2는 처음부터 운영 환경을 전제로 설계하며, Tauri 데스크톱 앱을 우선 제공하고 이후 모바일·웹 확장을 고려합니다.

> v1 웹 애플리케이션은 이 저장소에서 제거되었습니다. 현재 추적되는 코드는 모두 v2이며, 유지해야 할 v1 코드나 호환 계층은 없습니다.

### 현재 상태

| 영역 | 상태 |
| --- | --- |
| 공개 저장소 | Public / 오픈소스 운영 방향 |
| Figma Design System | 승인 완료 |
| Tauri + React/Vite | 실행 기반 CI 검증 완료 |
| Code Design System / AppShell | Stage 5 완료 |
| Rust/Axum API | 실행 기반 CI 검증 완료 |
| PostgreSQL 18 | migration + readiness CI 검증 완료 |
| 인증/권한 | OIDC/JWKS 요청 경계 기반 구현, membership RBAC 후속 |
| Collect | production-grade reference implementation 예정 |
| 보안 정리 | 추적 파일 정리 완료. Git history 정리는 별도 판단 |
| Branch protection | main/develop 적용 완료 |

자세한 현재 상태는 [docs/STATUS.md](docs/STATUS.md)를 봅니다.

### 기술 방향

| 영역 | 기준 |
| --- | --- |
| Desktop | Tauri 2 + React 19 + Vite + strict TypeScript |
| Server | Rust + Axum + Tokio + Tower + Serde |
| Database | PostgreSQL 18 + SQLx |
| Authentication | ZITADEL / OIDC Authorization Code + PKCE |
| Async | PostgreSQL transactional outbox + NATS JetStream + idempotent worker |
| Files | Private Cloudflare R2, 서버 권한 확인 후 접근 |
| Local state | SQLite draft/cache/outbox, 서버 데이터가 최종 기준 |
| Observability | tracing/OpenTelemetry 기반 |
| Design | 승인된 Figma Design System -> code tokens -> `packages/ui` |
| Git | short-lived branch -> `develop` -> `main` |

클라이언트는 PostgreSQL에 직접 연결하지 않습니다. **API 서버가 신뢰 경계와 권한 판단의 최종 책임자**입니다.

### Production-first 원칙

School Collect는 “일단 MVP로 만들고 나중에 운영 수준으로 고친다”는 전제를 사용하지 않습니다.

- tenant isolation, authorization, audit, migration, idempotency, recovery를 처음부터 최종 구조에 맞게 설계합니다.
- production DB migration에는 데모 학교·가상 사용자·예제 업무·예제 제출 데이터를 넣지 않습니다.
- 테스트 데이터가 필요하면 테스트 시작 시 생성하고 종료 시 rollback/truncate/container 폐기로 제거합니다.
- 저장소에 실제 업무 원본, 학생·교직원·학부모 개인정보, 운영 credential을 커밋하지 않습니다.
- 도메인상 필수인 reference data는 데모 데이터와 구분하여 migration으로 관리할 수 있습니다.
- UI의 권한 표시는 UX일 뿐이며 authorization을 대체하지 않습니다.

### 목표 구조

```text
apps/
└─ app/                     React/Vite + Tauri 2

services/
├─ api/                     Rust/Axum API
├─ worker/                  async worker
└─ migrator/                migration-only process

crates/
├─ domain/
├─ application/
├─ db/
├─ auth/
├─ contracts/
└─ observability/

packages/
├─ ui/
├─ schemas/
└─ config/

migrations/
└─ v2/
infra/
docs/
```

현재 실제 구현 상태와 목표 구조의 차이는 [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)와 [docs/STATUS.md](docs/STATUS.md)에 기록합니다.

### 디자인 기준

승인된 Figma 시안을 제품 UI의 기준으로 사용합니다.

- 밝고 compact한 업무형 desktop shell
- 한 화면에서 primary action은 명확하게 하나
- radius 최대 8px, 장식용 shadow 없음
- Card는 독립적인 정보/행동 객체일 때 사용
- 카드 면적은 정보량과 행동 밀도에 비례
- 같은 peer collection 안에서 한 항목만 Card로 승격하지 않음
- 목록 구분이 약하면 개별 카드 대신 `List Surface + List Row`
- 긴급도는 container 변경이 아니라 semantic color/status/typography로 표현
- Default / Loading / Empty / Error / Permission / Offline 상태를 화면 설계에 포함

상세 계약: [docs/DESIGN_SYSTEM.md](docs/DESIGN_SYSTEM.md)

### 협업

`main`은 검증된 승격 브랜치, `develop`은 다음 릴리스 통합 브랜치입니다.

```text
feat/* / fix/* / refactor/* / chore/* / docs/*
                    ↓ squash
                 develop
                    ↓ merge commit
                   main
                    ↓ sync
                 develop
```

긴급 수정은 `main`에서 `hotfix/*`로 분기한 뒤 `main`과 `develop`에 반영합니다. 자세한 규칙은 [CONTRIBUTING.md](CONTRIBUTING.md)를 따릅니다.

### 문서

- [현재 상태](docs/STATUS.md)
- [7단계 실행 계획](docs/V2_PLAN.md)
- [목표 아키텍처](docs/ARCHITECTURE.md)
- [아키텍처 결정](docs/ADR/0001-v2-platform.md)
- [Design System 계약](docs/DESIGN_SYSTEM.md)
- [보안 정책](SECURITY.md)
- [보안 baseline](docs/SECURITY_BASELINE.md)
- [브랜치 보호 체크리스트](docs/BRANCH_PROTECTION.md)
- [기여 가이드](CONTRIBUTING.md)

### 공개 저장소와 보안

이 저장소는 public 오픈소스 운영을 전제로 합니다. 따라서 “public이므로 숨긴다”가 아니라 **public에 있어도 되는 코드와 문서만 남기는 것**이 기준입니다.

과거 revision에 포함된 credential 형태 값과 업무 원본/파생 자료는 추적 파일에서 제거되었지만 Git history에는 남아 있습니다. 값 자체를 Issue, PR, 문서에 재게시하지 않습니다. 자세한 내용은 [SECURITY.md](SECURITY.md)를 봅니다.

라이선스는 별도 결정 후 저장소에 명시합니다. 라이선스 파일이 추가되기 전에는 사용·재배포 조건이 확정되었다고 가정하지 마세요.

---

## English

School Collect is an **open-source work platform** for collecting, validating, and automating recurring school administration workflows. v2 is designed from the start as a production system, with a desktop-first Tauri application and room for later mobile and web expansion.

> The v1 web application has been removed from this repository. Everything tracked here is v2; there is no v1 code or compatibility layer to maintain.

### Current status

| Area | Status |
| --- | --- |
| Repository | Public, intended for open-source development |
| Figma Design System | Approved |
| Tauri + React/Vite | Executable foundation validated in CI |
| Code Design System / AppShell | Stage 5 complete |
| Rust/Axum API | Executable foundation validated in CI |
| PostgreSQL 18 | Migrations and readiness validated in CI |
| Authentication/authorization | OIDC/JWKS request boundary implemented; membership RBAC follows |
| Collect | Planned as a production-grade reference implementation |
| Security cleanup | Tracked files cleaned; Git history remains a separate decision |
| Branch protection | Enabled on main and develop |

See [docs/STATUS.md](docs/STATUS.md) for the current verified state.

### Technology direction

| Area | Baseline |
| --- | --- |
| Desktop | Tauri 2 + React 19 + Vite + strict TypeScript |
| Server | Rust + Axum + Tokio + Tower + Serde |
| Database | PostgreSQL 18 + SQLx |
| Authentication | ZITADEL / OIDC Authorization Code + PKCE |
| Async | PostgreSQL transactional outbox + NATS JetStream + idempotent workers |
| Files | Private Cloudflare R2 with server-side authorization |
| Local state | SQLite draft/cache/outbox; server data remains authoritative |
| Observability | tracing/OpenTelemetry |
| Design | Approved Figma Design System -> code tokens -> `packages/ui` |
| Git | short-lived branches -> `develop` -> `main` |

Clients never connect directly to PostgreSQL. **The API server is the trust boundary and the final authorization authority.**

### Production-first principles

School Collect does not assume that production concerns can be deferred until after an “MVP.”

- Tenant isolation, authorization, auditability, migrations, idempotency, and recovery are designed into the target architecture.
- Production migrations do not seed demo schools, fake users, sample tasks, or sample submissions.
- Tests create only the data they need at runtime and remove it through rollback, truncation, or disposable infrastructure.
- Real work documents, student/staff/guardian personal data, and production credentials do not belong in the repository.
- Domain-required reference data may be migration-managed when it is product data rather than demo content.
- UI permission checks are UX only; they never replace server authorization.

### Target layout

```text
apps/
└─ app/                     React/Vite + Tauri 2

services/
├─ api/                     Rust/Axum API
├─ worker/                  async worker
└─ migrator/                migration-only process

crates/
├─ domain/
├─ application/
├─ db/
├─ auth/
├─ contracts/
└─ observability/

packages/
├─ ui/
├─ schemas/
└─ config/

migrations/
└─ v2/
infra/
docs/
```

Differences between the target layout and the currently integrated code are tracked in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and [docs/STATUS.md](docs/STATUS.md).

### Design baseline

The approved Figma work is the visual and interaction baseline.

- light, compact administrative desktop shell
- one clear primary action per screen
- radius <= 8px and no decorative shadows
- cards only for independent information or action objects
- card area proportional to information/action density
- do not promote only one item in a peer collection to a different container primitive
- prefer `List Surface + List Row` when dividers alone are too weak
- express urgency through semantic color, status, and typography
- design Default / Loading / Empty / Error / Permission / Offline states

See [docs/DESIGN_SYSTEM.md](docs/DESIGN_SYSTEM.md).

### Collaboration

`main` is the verified promotion branch and `develop` is the next-release integration branch.

Feature/fix/refactor/chore/docs branches are created from `develop` and squash-merged back to `develop`. Releases use a merge commit from `develop` to `main`, followed by a `main` -> `develop` sync. Hotfixes branch from `main`.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the complete workflow.

### Documentation

- [Current status](docs/STATUS.md)
- [7-stage execution plan](docs/V2_PLAN.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Architecture decision](docs/ADR/0001-v2-platform.md)
- [Design System contract](docs/DESIGN_SYSTEM.md)
- [Security policy](SECURITY.md)
- [Security baseline](docs/SECURITY_BASELINE.md)
- [Branch protection checklist](docs/BRANCH_PROTECTION.md)
- [Contributing](CONTRIBUTING.md)

### Public repository and security

The repository is intentionally public. The standard is therefore not to hide the repository, but to ensure that only code and documentation suitable for public distribution remain in it.

Legacy credentials and real work/source artifacts found in Git history are handled as a separate security cleanup. Never copy their values into issues, pull requests, logs, or documentation.

The project license will be declared separately. Until a license file is added, do not assume redistribution or reuse terms have been granted.
