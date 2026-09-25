# v2 API 계약 기준

이 문서는 기능 브랜치가 공통으로 사용하는 서버 계약의 출발점입니다. 실제 OpenAPI 문서는 API가 실행될 때 `/openapi.json`으로 확인합니다.

## 공개 endpoint

- `GET /health`: 프로세스 생존 확인
- `GET /ready`: PostgreSQL readiness 확인
- `GET /openapi.json`: 계약 문서

업무 endpoint는 `Authorization: Bearer <access-token>`을 요구합니다. staging과 production에서 인증 실패를 허용하는 fallback은 없습니다.

## 업무 endpoint

tenant 범위 endpoint는 `X-Tenant-Id`를 함께 받습니다. 이 값은 조회 조건일 뿐이고,
검증된 actor의 membership이 없으면 `403 forbidden`입니다.

| method | 경로 | 권한 | 용도 |
| --- | --- | --- | --- |
| `GET` | `/v1/auth/principal` | 인증됨 | 검증된 token의 actor 확인 |
| `GET` | `/v1/session` | 인증됨 | 사용자와 소속 목록 |
| `GET` | `/v1/tenants` | 인증됨 | 접근 가능한 학교 목록 |
| `POST` | `/v1/tenants` | 인증됨 | 학교 등록(생성자는 `admin`) |
| `GET` | `/v1/members` | `admin`, `coordinator` | 구성원과 역할 |
| `GET` | `/v1/collects` | `viewer` 이상 | 수합 목록(내 제출 상태 포함) |
| `POST` | `/v1/collects` | `admin`, `coordinator` | 수합 생성(항목·대상 포함) |
| `GET` | `/v1/collects/{id}` | `viewer` 이상 | 상세(항목, 내 제출, 진행 수치) |
| `GET` | `/v1/collects/{id}/status` | `admin`, `coordinator` | 제출 현황 명단 |
| `POST` | `/v1/collects/{id}/publish` | `admin`, `coordinator` | 배포 |
| `POST` | `/v1/collects/{id}/close` | `admin`, `coordinator` | 마감 |
| `PUT` | `/v1/collects/{id}/submission` | `admin`, `coordinator`, `contributor` | 임시 저장 |
| `POST` | `/v1/collects/{id}/submission/submit` | `admin`, `coordinator`, `contributor` | 제출 |
| `GET` | `/v1/assignments` | `viewer` 이상 | 내가 제출할 수합 |

`POST /v1/collects`의 `items`는 항목 정의(`key`, `label`, `required`)이고 `key`는
`^[a-z][a-z0-9_]*$`이며 collect 안에서 유일해야 합니다. `assigneeUserIds`를 비우면
`viewer`를 제외한 전체 구성원이 대상이 됩니다. 대상은 서버가 `memberships`로 다시
확인하므로 다른 학교의 사용자를 지정할 수 없습니다.

## 인증 결과

서버는 token의 `iss`, `sub`, signature, `aud`, `exp`, `nbf`를 검증합니다. `sub`는 `users(issuer, subject)`와 연결되고, tenant와 role은 `memberships`에서 조회합니다.

`X-Tenant-Id`가 사용되는 endpoint에서도 해당 값만으로 권한을 부여하지 않습니다. 검증된 actor가 그 tenant의 membership을 가지고 있어야 합니다.

## 오류 형태

```json
{
  "code": "forbidden",
  "message": "you do not have access to this resource",
  "request_id": "req_01..."
}
```

주요 code:

- `unauthorized`: token 누락 또는 검증 실패
- `forbidden`: membership 또는 role 부족
- `tenant_not_found`: 접근 가능한 tenant가 아님
- `invalid_state_transition`: 업무 상태 전이 불가
- `version_conflict`: optimistic version 충돌
- `idempotency_conflict`: 같은 key에 다른 mutation payload가 사용됨

## Collect 기준 흐름

1. 관리자 또는 coordinator가 draft collect와 item, 대상(assignment)을 함께 만듭니다.
2. publish가 상태를 `published`로 바꿉니다. 대상이 비어 있었다면 이 시점에 기본
   대상으로 채웁니다.
3. 대상 구성원은 자기 assignment에 대해 draft submission을 저장합니다. 저장하면
   assignment는 `started`가 됩니다.
4. 제출하면 submission은 `submitted`, assignment는 `submitted`가 됩니다.
5. 관리자 또는 coordinator가 collect를 `closed`로 전환합니다.

`closed` 이후 제출과 변경은 거부합니다. draft 저장은 `expected_version`을 사용하고, 성공 시 version을 1 증가시킵니다.

## 데이터 계약

현재 migration이 정의하는 공용 테이블은 `tenants`, `users`, `memberships`, `collects`, `collect_items`, `collect_assignments`, `collect_submissions`, `audit_events`, `outbox_events`입니다. 팀 기능은 이 명칭과 tenant FK를 임의로 바꾸지 않고 별도 ADR로 변경 이유를 남깁니다.
