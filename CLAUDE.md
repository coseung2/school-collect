# School Collect - 학교 업무수합 플랫폼

## 스택
- Next.js 16 (App Router)
- Supabase (PostgreSQL) + Supabase JS SDK
- Tailwind CSS

## 개발 명령어
- `npm run dev` — 개발 서버
- `npm run build` — 빌드

## DB 테이블 생성 방법
`supabase-schema.sql`을 Supabase Dashboard SQL Editor에 붙여넣고 실행.
(봄봄봄이랑 같은 방식 — Prisma 없이 SDK만 씀)

## 중요한 파일
- `src/lib/supabase.ts` — Supabase 클라이언트
- `src/lib/types.ts` — 공통 타입 정의
- `supabase-schema.sql` — DB 스키마 SQL

## 데이터 흐름
- 서버 API routes는 `getServiceSupabase()`로 service_role key 사용
- 클라이언트 페이지는 fetch로 API 호출
- 입력 폼만 클라이언트 컴포넌트, 나머진 서버 컴포넌트

## HWPX 메일머지 워크플로우
- `_agent_작업/메일머지_워크플로우/` 참고
- `school-mailmerge` 스킬 로드: `skill_view(name='school-mailmerge')`
- 신체검사 PDF OCR → CSV 검증 → HWPX 개인별 생성
- 재사용: 템플릿만 바꾸고 같은 파이프라인 사용 가능

## 메일머지 웹앱 기능
- `/mailmerge` — 메일머지 대시보드 (템플릿 목록 + 실행 기록)
- `/mailmerge/templates/new` — 새 템플릿 등록
- `/mailmerge/templates/[id]` — 템플릿 상세 (필드 목록 + 실행 기록)
- `/mailmerge/runs/new` — 새 메일머지 실행 생성 (템플릿 선택 + 대상 정보)
- `/mailmerge/runs/[id]` — 실행 상세 (상태 추적 + 데이터 파일 경로)
- DB 테이블: `mailmerge_templates`, `mailmerge_runs` (supabase-schema.sql 참고)
- 실제 HWPX 생성은 Hermes 에이전트가 처리 (웹앱은 기록/관리)

배포
Vercel에 배포 시 환경변수 설정:
- NEXT_PUBLIC_SUPABASE_URL
- NEXT_PUBLIC_SUPABASE_ANON_KEY
- SUPABASE_SERVICE_ROLE_KEY
