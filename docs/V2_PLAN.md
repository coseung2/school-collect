# School Collect v2: 7단계 실행 계획

기준일: 2026-09-21. 상위 이슈: #1. 이번 범위의 끝은 Collect reference feature이며, 모든 모듈 이관이 아닙니다.

## 완료 상태의 의미

`계획` / `구현 중` / `검증 중` / `리뷰 대기` / `완료`를 구분합니다. 설정 파일·스캐폴드·목업은 실제 실행 및 통합 검증을 대신하지 않습니다. 각 단계는 PR, 검증한 commit SHA, 실행 명령/결과, 리뷰 기록, 남은 위험을 남깁니다. 현재 상태는 `STATUS.md`를 참조합니다.

## 순서와 게이트

| 단계 | 이슈 | 결과물 | 완료 게이트 |
| --- | --- | --- | --- |
| 1. 기준점·보안 | #2 | 안전한 예제, ignore, 생성파일 정리, 이력/자료 점검 | 자격증명·개인정보 검토 및 보존 결정 |
| 2. 공동작업 | #3 | develop, 규칙, templates, owners, CI | 보호 설정과 실제 차단·리뷰 확인 |
| 3. 실행 기반 | #4 | Rust/TS workspace, API 계약, 개발환경 | 재현 가능한 install/build/test, 빈 DB migration |
| 4. Figma | #5 | tokens, components, Desktop/Mobile Shell | 실제 파일 검수와 헤드 승인 |
| 5. AppShell | #6 | 코드 DS, 실제 Tauri 창/라우팅/상태 | native smoke 및 UI/접근성 검수 |
| 6. Auth/Data | #7 | 인증/권한/저장/작업/초안/운영 | 보안 부정 테스트, 장애/복원 검증 |
| 7. Collect | #8 | 기준 사용자 흐름 전체 | develop 통합 SHA 검증 후 승격 승인 |

1 -> 2 -> 3 순으로 진행합니다. 3단계의 아키텍처 결정 후 4단계 디자인은 일부 병행할 수 있습니다. 5는 3/4에, 6의 서비스 기반은 3에, 7은 4/5/6에 의존합니다. 명확한 선행 조건 없이 다음 단계를 완료 처리하지 않습니다.

## 1. 기준점·보안

현재 v1 기준 SHA는 `9ac77a84b100ac1a0a2c36ddfc1292410bff1582`입니다. 원본 소스와 기존 브랜치를 유지하며 이력 재작성은 하지 않습니다. Supabase는 폐기되었으므로 기존 DB 백업·접속을 계획에 넣지 않습니다.

예제값 삭제, generated 파일 제거, 신규 실데이터 반입 금지부터 적용합니다. 기존 업무 원본과 Git 과거 이력은 별도로 검토합니다. 유효한 credential인지 확인되지 않은 값은 유출 확정으로 표현하지 않습니다. `v1-web-final` 같은 보존 태그는 보안 점검 및 보존 정책 승인 후 생성합니다.

## 2. 공동작업

`feat/fix/refactor/chore/docs -> develop -> main`. 상세 merge 방법은 CONTRIBUTING을 따릅니다. main 승격 후 main -> develop 동기화를 포함합니다. hotfix만 main에서 분기합니다.

초기 CI는 repository-checks입니다. full-history secret scanning과 native/API 테스트를 이 작은 검사로 대체하지 않습니다. 실제 branch protection 설정, reviewer 지정, 권한 차단 시험까지 끝나야 이 단계가 완료됩니다.

## 3. 실행 기반

목표는 modular monolith API + 별도 worker 프로세스입니다. 불필요하게 여러 마이크로서비스로 나누지 않으며, 인증 서비스 ZITADEL과 DB는 별도 책임을 가집니다. 모노레포 경로와 의존 경계는 ARCHITECTURE에 정의합니다.

후속 PR은 workspace/toolchain -> health/readiness/API 계약 -> DB 역할·migration -> CI 순으로 작게 나눕니다. Cargo/pnpm lockfile과 도구 버전을 실제 설치/빌드 결과로 고정합니다. PostgreSQL 18.x 최신 보안 패치를 검증하여 사용하며, 현재 검증한 upstream 버전은 18.6입니다.

서비스를 실행하지 않은 상태에서 가짜 readiness 성공을 반환하지 않습니다. API liveness와 DB readiness는 분리합니다. 운영 계정 없이도 로컬 합성 데이터로 검증 가능해야 합니다.

## 4. Figma

사용자가 선택한 팀에 Design System과 Product 파일을 만듭니다. 변수 naming, 상태, interaction, compact/comfortable density, Desktop/Mobile navigation을 정의합니다. 색상·폰트·최종 스타일은 헤드의 승인 대상입니다.

Figma 설계와 코드 릴리스의 연결은 승인된 token manifest/variant 대응표로 관리합니다. 자동 동기화나 Library publish 권한을 전제하지 않습니다. Figma URL과 검수 기록이 있어야 완료입니다.

## 5. 코드 DS / AppShell

토큰, Button/Input/FormField/Dialog/Table 등 기준 컴포넌트, 컴포넌트 카탈로그, routing/theme/navigation/error/offline 상태를 구현합니다. UI primitive에는 학교 업무 API 호출을 넣지 않습니다.

Windows를 1차 검증 대상으로 제안하되 OS별 실제 결과를 기록합니다. macOS/Linux 지원을 문서 선언만으로 완료 처리하지 않습니다. 모바일 UI 전체는 범위 밖이지만 로그인 callback, OS 보안 저장, 파일 접근, 키보드/safe-area, 플러그인 지원을 작은 spike로 검증합니다.

## 6. 인증·데이터·운영 기반

- 6A: OIDC/ZITADEL 실제 로그인, 외부 브라우저/PKCE/state/nonce, 검증된 redirect, API token 검증, 세션 종료.
- 6B: 학교 membership/RBAC와 RLS, pool tenant scope, runtime/migrator 분리, audit 최소수집, migration 통합 테스트.
- 6C: R2 private 파일 접근, 제한된 업로드, 수명 제한 다운로드 URL, 삭제/보존 정책.
- 6D: DB 변경과 outbox INSERT를 한 트랜잭션으로 기록. relay -> NATS -> idempotent worker. ACK는 처리 커밋 후, 재시도/실패함/관측성 검증.
- 6E: SQLite는 사용자별 초안/outbox만. 서버가 최종 기준. version/idempotency key, 충돌 UI, 로그아웃/만료 데이터 정리. 전면 offline replication은 범위 밖.
- 6F: staging/production 분리, tracing/metrics/log redaction, 실제 복원 테스트, 릴리스 서명/업데이트 검증.

운영 서버, 도메인, Cloudflare/R2 계정, ZITADEL bootstrap 권한, 플랫폼 서명키는 아직 지정되지 않았습니다. 대상 확정 전 비용 발생 자원을 만들지 않습니다. 초기 복구 목표 제안은 RPO 15분/RTO 4시간이며 소유자의 운영 요구와 예산으로 확정하고 복구 훈련에서 측정합니다. 이 수치는 달성 사실이 아닙니다.

## 7. Collect reference feature

관리자 생성 -> 배포 -> 교사 초안 -> 제출 -> 담당자 현황 -> 마감 -> 결과/내보내기를 vertical slice로 구현합니다. 서버/DB/UI/테스트를 작은 사용자 흐름 단위로 함께 리뷰합니다.

학교 A/B 격리, 교사/담당자/관리자 권한, 마감 이후 제출, 중복 전송, 동시 수정, 네트워크 단절, 재시작 초안 복구, export 재전달을 시험합니다. 시간 판단은 서버 기준이며 타임존 정책을 명시합니다. 상태 전이는 서버에서 검증합니다.

전체 흐름의 native 테스트와 develop 통합 후보 SHA가 승인되면 develop -> main 승격 PR을 엽니다. 코드가 빌드된다는 이유만으로 운영 데이터나 실제 사용자에게 배포하지 않습니다.

## 공통 운영 원칙

실데이터 사용 금지, secrets 출력 금지, 작성자 외 리뷰, 변경마다 검증 증거, 운영 변경 별도 승인. 기존 자료의 삭제/이력 재작성은 백업·접근권한·공개 영향까지 확인한 별도 변경입니다.
