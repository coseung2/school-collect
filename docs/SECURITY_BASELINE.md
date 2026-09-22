# 보안 baseline

기준일: 2026-09-22. 추적 이슈: #2.

## 저장소 운영 방향

repository는 **public 유지**가 사용자 의도입니다. 보안 목표는 private 전환이 아니라 공개 저장소에서 secret, 실제 업무자료, 개인정보를 제거하고 재유입을 막는 것입니다.

## 확인된 과거 revision 위험

값 자체는 이 문서에 기록하지 않습니다.

확인된 항목:
- 과거 `.env.example`에 실제처럼 보이는 외부 서비스 endpoint/token/DB URL 형태 값
- 과거 local CLI state에 linked project metadata, connection URL, project reference 정보
- `_agent_작업` 아래 실제 업무에서 생성된 spreadsheet/CSV/Google Docs·Sheets source metadata 및 derived artifact
- 일부 업무 artifact에 학교 운영정보/교직원 이름이 포함될 가능성 또는 실제 확인 사례

이 사실은 “모든 값이 현재 유효하다”는 의미는 아닙니다. 그러나 public Git history에 있었던 credential은 안전하다고 가정하지 않고 외부 시스템에서 폐기/회전해야 합니다.

## 현재 HEAD 정리

PR #10에서 `.env.example` 값, 과거 local CLI state, `tsconfig.tsbuildinfo`를 제거하고 `.gitignore`를 강화했습니다.

이후 소유자 결정에 따라 v1 웹 애플리케이션과 `_agent_작업` 업무자료를 추적 대상에서 전부 제거했습니다. 따라서 **현재 HEAD에는 실제 업무자료와 retired stack 설정이 남아 있지 않습니다.**

이 작업은 추적 파일 노출을 없애지만 과거 Git object를 제거하지 않습니다.

## 남은 security gate

1. 외부 credential 폐기/회전 확인
2. Git history rewrite 대상 path/blob 확정
3. rewrite 전 백업/협업자 영향 확인
4. history rewrite + force update를 별도 승인된 작업으로 실행
5. branch/tag/fork/cache 등 공개 사본 한계 기록
6. 이후 full-history secret/data scan 재검증

추적 파일 분류·삭제(기존 2·3번)는 완료되었습니다. history rewrite는 아직 실행 완료로 기록하지 않습니다.

## Production/test data 원칙

production migration과 repository에 demo school/user/task/submission seed를 상시 두지 않습니다.

테스트 데이터는 test runtime에 생성하고 다음 중 하나로 제거합니다.
- transaction rollback
- truncate/teardown
- disposable database/schema/container

제품에 필수인 reference data만 demo data와 구분하여 migration 관리할 수 있습니다.

## Guard 한계

repository guard는 명확한 금지 경로/패턴의 재유입 방지 장치입니다.

통과해도 다음을 자동 보장하지 않습니다.
- full Git history가 깨끗함
- binary office file에 개인정보가 없음
- 모든 secret 패턴을 탐지함
- 외부에서 credential이 폐기됨
- GitHub 외부 복사본이 삭제됨

Stage 1은 위 보안 게이트가 실제로 닫혀야 완료입니다.
