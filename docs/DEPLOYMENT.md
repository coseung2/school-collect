# 실행과 배포

School Collect는 서버(API·migrator·worker)와 데스크톱 앱으로 나뉩니다. 이 문서는
지금 실제로 실행 가능한 부분과 소유자 결정이 필요한 부분을 구분합니다.

## 지금 실행 가능한 것

- 서버 이미지: `infra/api.Dockerfile`이 `school-collect-api`,
  `school-collect-migrator`, `school-collect-worker`를 빌드합니다.
- 배포 구성: `infra/compose.deploy.yml`이 `api`(상시), `migrator`(1회성
  profile), `worker`를 정의합니다. 값은 모두 환경변수로만 들어갑니다.
- 상태 확인: `/health`는 프로세스 생존, `/ready`는 PostgreSQL 연결과 v2
  스키마 적용 여부를 함께 확인합니다.
- 마이그레이션: `school-collect-migrator`만 스키마를 변경합니다. API 기동의
  부수 효과로 스키마가 바뀌지 않습니다.
- 데스크톱 앱: `pnpm --filter @school-collect/app tauri build`로 설치 파일을
  만들고, `tauri dev`로 개발 실행합니다.
- 로컬 개발 스택: `infra/compose.dev.yml`이 개발자별 `postgres`(18), `nats`,
  일회성 `migrator`, `api`, `worker`를 정의합니다.

## 서버 실행 순서

```sh
docker compose -f infra/compose.deploy.yml build
docker compose -f infra/compose.deploy.yml --profile migrate run --rm migrator
docker compose -f infra/compose.deploy.yml up -d api worker
curl -fsS https://<api-host>/ready
```

필수 환경변수:

- `APP_ENV` = `staging` 또는 `production`
- `APP_CORS_ORIGIN` = 실제 클라이언트 origin (loopback 기본값은 development 전용)
- `DATABASE_URL` = PostgreSQL pooler 주소
- `OIDC_ISSUER_URL`, `OIDC_AUDIENCE` (`OIDC_JWKS_URL`은 discovery가 없는
  provider용 선택값)

값은 Infisical에서 실행 시점에 주입하고, 이미지나 저장소에 넣지 않습니다.

## 로컬 개발 (개발자별 Docker)

개발 환경은 공용으로 두지 않습니다. 각 개발자가 자기 기계에서 서버 스택 전체를
컨테이너로 띄우므로 개발 DB와 개발 데이터가 팀원 사이에 섞이지 않습니다.

```sh
docker compose -f infra/compose.dev.yml up -d
```

- `postgres`(18)와 `nats`가 뜨고, 일회성 `migrator`가 `migrations/v2`를 적용한 뒤
  종료하며, `api`는 그 다음에 시작합니다.
- API는 `http://127.0.0.1:3000`이고 `/health`와 `/ready`를 제공합니다.
- 상태 확인은 `docker compose -f infra/compose.dev.yml ps`, 종료는 `... down`,
  개발 데이터까지 초기화하려면 `... down -v`입니다.
- 서버 이미지를 처음 빌드하므로 첫 `up`은 몇 분 걸립니다.

기본값은 `APP_AUTH_MODE=disabled`이고, 이때 로컬 API는 고정 development 주체로
동작합니다. 실제 로그인까지 확인하려면 `APP_AUTH_MODE=oidc`, `OIDC_ISSUER_URL`,
`OIDC_AUDIENCE`를 지정합니다. `development`가 아닌 환경에서는 인증을 끈 채로
기동할 수 없습니다.

## 클라이언트 실행

```sh
pnpm --filter @school-collect/app dev        # 웹 미리보기
pnpm --filter @school-collect/app tauri dev  # 데스크톱 창
```

빌드 시 필요한 공개 값은 `VITE_API_BASE_URL`, `VITE_SUPABASE_URL`,
`VITE_SUPABASE_ANON_KEY`입니다. 모두 공개 값이며 서버 자격증명은 절대 `VITE_`
변수로 들어가지 않습니다.

## 아직 결정되지 않은 것

아래 항목은 저장소 작업만으로 정할 수 없어 소유자 결정이 필요합니다. 결정 전에는
임의로 외부 자원을 만들지 않습니다.

- **호스팅 위치**: 어느 서버·컨테이너 플랫폼에 올릴지
- **환경 분리**: 현재 dev·staging·prod Infisical 라벨이 같은 Supabase
  프로젝트를 가리킵니다. staging용 프로젝트를 새로 만들려면 Supabase 무료
  한도(계정당 무료 프로젝트 2개)를 넘어야 하므로, Pro 업그레이드 또는 무료
  슬롯 정리 결정이 먼저 필요합니다. 결정 전에는 외부 자원을 만들지 않습니다.
- **도메인과 TLS**: API 호스트, 인증서, 그리고 데스크톱 CSP `connect-src`에 넣을
  실제 origin
- **비밀값 주입**: 대상 호스트에서 Infisical Machine Identity를 쓸지 여부
- **DB 역할 분리**: runtime 역할과 migration 역할 분리 및 최소 권한
- **데스크톱 서명·업데이트**: 코드 서명 인증서와 업데이트 채널
- **백업·복구**: 백업 주기와 실제 복구 훈련(RPO/RTO 기록)

## 확인 범위

- 실제 Supabase 프로젝트와 실제 PostgreSQL에 대해 로그인부터 제출·마감까지
  `services/api/tests/collect_flow_e2e.rs`가 검증합니다.
- 데스크톱 UI 흐름(로그인 → 학교 등록 → 수합 생성 → 배포 → 초안 저장 → 제출)은
  로컬 API와 dev 서버를 띄운 상태에서 실제 브라우저로 확인했습니다.
- `infra/api.Dockerfile` 이미지 빌드는 이 개발 머신에 Docker 데몬이 없어 로컬에서
  실행하지 못했습니다. 대신 PR CI의 `server-image` 작업이 이미지를 빌드하고,
  그 이미지로 PostgreSQL에 마이그레이션을 적용한 뒤 `/ready` 200까지 확인합니다.
