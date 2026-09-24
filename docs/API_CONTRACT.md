# v2 API 계약 기준

이 문서는 기능 브랜치가 공통으로 사용하는 서버 계약의 출발점입니다. 실제 OpenAPI 문서는 API가 실행될 때 `/openapi.json`으로 확인합니다.

## 공개 endpoint

- `GET /health`: 프로세스 생존 확인
- `GET /ready`: PostgreSQL readiness 확인
- `GET /openapi.json`: 계약 문서

업무 endpoint는 `Authorization: Bearer <access-token>`을 요구합니다. staging과 production에서 인증 실패를 허용하는 fallback은 없습니다.

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

1. 관리자 또는 coordinator가 draft collect와 item을 생성합니다.
2. publish 시점에 assignment를 만들고 상태를 `published`로 변경합니다.
3. contributor는 자기 assignment에 대해 draft submission을 저장합니다.
4. contributor가 제출하면 submission은 `submitted`가 됩니다.
5. 관리자 또는 coordinator가 collect를 `closed`로 전환합니다.

`closed` 이후 제출과 변경은 거부합니다. draft 저장은 `expected_version`을 사용하고, 성공 시 version을 1 증가시킵니다.

## 데이터 계약

현재 migration이 정의하는 공용 테이블은 `tenants`, `users`, `memberships`, `collects`, `collect_items`, `collect_assignments`, `collect_submissions`, `audit_events`, `outbox_events`입니다. 팀 기능은 이 명칭과 tenant FK를 임의로 바꾸지 않고 별도 ADR로 변경 이유를 남깁니다.
