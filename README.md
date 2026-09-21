# School Collect v2

학교 업무수합 플랫폼을 **Tauri 데스크톱 우선 + 향후 모바일 확장** 구조로 재구축합니다.

> 현재 이 브랜치는 **저장소·협업 기반 구축 단계**입니다. Tauri 앱, Rust API, 로그인, DB, Collect v2가 이미 구현되었다는 뜻이 아닙니다. 기존 `src/`와 Next.js manifest는 아직 v1 코드이며, 폐기된 Supabase 프로젝트는 사용하지 않습니다.

## 확정 방향

| 영역 | 기준 |
| --- | --- |
| 클라이언트 | Tauri 2, React, Vite, TypeScript; 모바일 플랫폼 어댑터 고려 |
| 서버 | Rust, Axum, SQLx, PostgreSQL 18 |
| 인증 | ZITADEL, OIDC Authorization Code + PKCE |
| 파일/엣지 | Cloudflare DNS/WAF 및 private R2 |
| 비동기 | PostgreSQL transactional outbox, NATS JetStream, idempotent worker |
| 로컬 저장 | SQLite 초안/outbox; 서버가 최종 데이터 기준 |
| 디자인 | Figma 디자인 시스템과 저장소 토큰 계약 |
| Git | 작업 브랜치 -> develop 통합 -> main 승격 |

## 시작 문서

- [7단계 계획](docs/V2_PLAN.md) / 상위 추적 이슈: #1
- [현재 실행 상태](docs/STATUS.md)
- [협업 규칙](CONTRIBUTING.md) / [브랜치 보호 설정](docs/BRANCH_PROTECTION.md)
- [목표 아키텍처](docs/ARCHITECTURE.md) / [아키텍처 결정](docs/ADR/0001-v2-platform.md)
- [Figma 디자인 시스템 계약](docs/DESIGN_SYSTEM.md)
- [보안 정책](SECURITY.md) / [초기 보안 점검 기록](docs/SECURITY_BASELINE.md)

## 현재 실행 가능한 검사

Git checkout과 Python 3.11 이상이 필요합니다. 외부 Python 패키지는 사용하지 않습니다.

```sh
python3 -m unittest discover -s scripts/tests -v
python3 scripts/check_repository.py
```

이 검사는 저장소 규칙용입니다. v2 앱 빌드·native 실행·인증·DB 통합 테스트를 대신하지 않습니다. v2 실행 명령과 Cargo/pnpm lockfile은 3단계 구현 PR에서 추가합니다.

## 범위

1차 완료 지점은 **Collect reference feature**입니다. 다른 업무 모듈 전체 이관과 모바일 정식 출시는 이후 범위입니다. 업무 원본·학생/교직원/학부모 실데이터는 코드 저장소와 테스트 fixture에 넣지 않습니다.
