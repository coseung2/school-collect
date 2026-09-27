# 팀 작업 순서와 공통 기반

기준일: 2026-09-27 (Asia/Seoul). 전체 분류는 [V2_PLAN.md](V2_PLAN.md), 기능별 범위·완료 조건은 [학교업무](SCHOOL_WORK_PLAN.md)와 [개인기능](PERSONAL_PLAN.md), 통합 상태는 [STATUS.md](STATUS.md)를 따릅니다.

## 다음 작업 묶음

| 순서 | 학교업무 | 개인기능 | 공통 조정 |
| --- | --- | --- | --- |
| 1 | 완료: S-01 PR #24 통합(`f573bfb`) | 완료: P-01 PR #25 통합(`11a0af0`)·실제 창 검증 | 완료: C-01 학교/개인 진입 경계 통합(설정·offline 세부는 남음) |
| 2 | 완료: S-02 구성원 초대·합류(PR #28, `9416c6c`) | 완료: P-02 확장·현재 화면 등록(PR #30, `795dc26`) | C-02: 기존 auth/RBAC 부정 테스트·관측 보강 |
| 3 | 완료: S-03 편집(PR #41), S-06 relay+소비 worker(PR #43, PR #79), S-05 세션·초안(PR #45·#46) | 완료: P-03 엔진(PR #32·#33), P-04 행렬(PR #35), P-06 감시(PR #37), P-07 추천(PR #51) | C-03 서버 운영, C-05 대상별 배포 검증 |
| 4 | 완료: S-07 결과 내보내기·화면·역할 4종 검증(PR #48·#50·#53), S-04 첨부(PR #58·#59·#60·#69·#70·#71) / 남음: 실제 R2 bucket 연결(소유자 자원) | P-05 데이터 연결 | C-04 보안·협업 게이트를 해당 출시 전에 확인 |

선후 관계가 없는 기능은 병렬로 진행할 수 있습니다. 개인기능의 학교 데이터 연결(P-05)만 필요한 학교 API 계약을 선행 조건으로 가집니다. 출시일은 아직 확정하지 않았습니다.

## 공통 작업

| ID | 작업 | 현재 / 남은 것 | 완료 조건 |
| --- | --- | --- | --- |
| C-01 | Design System·AppShell·영역 경계 | 기본 DS/AppShell 통합; PR #54에서 설정·narrow 결함 수정; PR #66에서 Select·Checkbox 추가와 제품 화면 적용. 남은 것은 Dialog·Sheet·Toast(Figma 승인 필요)와 Library/Code Connect | 개인 화면은 로그인/학교/API 장애에 막히지 않음, 학교 화면은 membership 요구, 기기/학교 설정 구분, 키보드·narrow·offline·permission 검증 (2026-09-27 실제 창에서 확인) |
| C-02 | 서버 인증·권한·오류·관측 / 완료(PR #39, `7285981`) | OIDC/JWKS·user upsert·membership RBAC·request ID 구현 + 키 회전/장애/캐시, 네 역할, 요청 ID·redaction 부정 테스트, 요청 ID span과 `/metrics` | 남은 것: RLS는 필요성 판단 후속(적용 완료로 표시하지 않음), 운영 수집기 연결 |
| C-03 | 학교 서버 환경·데이터·복구 | 로컬 Docker/배포 구성 있음; PR #62로 백업·복구 drill 스크립트를 CI에서 매번 실행(schema·migration·테이블별 행 수·canary 대조). 운영 연결 미완 | staging/prod 분리, HTTPS/CORS/CSP, 실제 runtime/migrator 역할·자격증명 생성과 연결(입력 분리와 역할 검사는 저장소 안에서 완료), Infisical 주입, 운영 DB 대상 drill과 RPO/RTO |
| C-04 | 저장소 보안·협업 | HEAD 자료 정리·보호 설정 과거 검증 기록 있음; PR #64로 전체 이력 스캔(155 커밋, HEAD 0건)과 rewrite 범위 후보 3묶음 기록 | 나머지 credential 폐기 확인, 승인된 history rewrite 실행·공개 사본 한계, 미검증 merge 차단, 협업자 추가 시 approval/code-owner 강화 |
| C-05 | 앱 설치·서명·업데이트·플랫폼 | PR #54에서 통합 SHA native 검증; PR #63으로 Windows 설치 파일(NSIS, 현재 사용자 설치)을 CI에서 매번 생성·검사(서명 없음); PR #73으로 설치 → 업그레이드 → 제거를 CI에서 확인. 남은 것은 서명 인증서·업데이트 채널(소유자 결정) | 변경 통합 SHA의 native 검증, 승인된 서명/업데이트·복구; Linux/macOS/mobile은 해당 대상을 제공하기 전 검증 |

C-03은 학교 서버에 의존하는 기능의 운영 게이트입니다. C-05의 Windows 검증과 모바일 검증은 구분합니다. Figma Library/Code Connect·추가 컴포넌트·interaction/accessibility는 C-01 후속입니다.

## 이미 통합된 기반

- PR #17: `packages/ui` tokens/components와 AppShell.
- PR #19: 실제 provider 토큰 검증, issuer/subject -> user 연결, membership 기반 권한, 수합 기본 상태 전이·version 충돌, request ID 계층.
- PR #20: 개발자별 PostgreSQL/NATS/API/migrator/worker Docker 구성. 이후 S-06(PR #43)이 outbox relay를, S-04 3차(PR #60)가 첨부 만료 sweep을 worker에 추가했습니다. 이벤트를 멱등 처리하고 commit 뒤에만 ACK하는 durable 소비 worker를 5차 검토에서 추가했습니다(알림 발송 채널은 아직 없어 소비 효과는 처리 기록까지입니다).

외부 브라우저 PKCE와 R2 실제 저장소 연결은 별도로 완료해야 합니다(OS 보안 저장소·SQLite 초안·outbox/JetStream relay·첨부 metadata/API/sweep은 완료). provider 선택은 [ADR-0002](ADR/0002-supabase-identity-and-database.md)를 따릅니다.

## 변경 소유권과 통합

- 새 작업은 최신 `origin/develop`에서 분기해 `develop` 대상 PR로 제안합니다.
- S-01(PR #24)과 P-01(PR #25 및 로컬 `c31b9d9`)은 `apps/app` 변경이 겹칩니다. 병합 후보·소유 파일을 먼저 비교하고 한 브랜치 전체를 다른 기능 완료 근거로 사용하지 않습니다.
- 공통 AppShell·navigation·token/component 계약은 C-01에서 확정해 종속 기능에 전달합니다.
- 학교업무는 domain/application/DB/API/학교 화면, 개인기능은 로컬 화면/native adapter/확장 범위를 소유합니다. 공유 UI와 P-05의 데이터 계약만 명시적으로 조율합니다.
- 학교 API/DB와 개인기능 local-only 변경을 같은 PR에 묶지 않습니다. 이미 섞인 후속 변경은 통합 전에 분리·대조합니다.
- PR마다 작업 ID, 소유 파일, 선행 PR/SHA, 검증·미검증 범위, 테스트 데이터 정리, 되돌리기를 기록합니다.
- 운영 배포/migration, credential 회전, DNS/유료 자원/서명키 변경, 파괴적 데이터/history 작업은 별도 명시적 권한이 필요합니다.
