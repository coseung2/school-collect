# 🔬 karpathy-compact 분석 리포트: school-collect

**분석일:** 2026-05-28  
**분석자:** OWL (자동 분석)  
**프레임워크:** Andrej Karpathy의 LLM 코딩 가이드라인 6원칙

---

## 📊 프로젝트 개요

| 항목 | 내용 |
|------|------|
| **프로젝트명** | school-collect (학교 업무수합 플랫폼) |
| **경로** | `/mnt/c/Users/심보승/Desktop/Obsidian Vault/school-collect/` |
| **스택** | Next.js 16 + React 19 + TypeScript + Supabase |
| **주요 도메인** | 수합(Collect), 월중계획(Monthly Plan), 메일머지(Mailmerge), 구입요청(Purchase) |
| **총 소스 파일** | 26개 (API 10개 + 페이지 14개 + 라이브러리 2개) |
| **총 라인 수** | 약 3,500줄 (src/ 기준) |
| **DB 스키마** | 8개 테이블, SQL 파일 196줄 |

---

## 🏆 종합 등급: **B+ (양호)**

| 원칙 | 등급 | 핵심 평가 |
|------|------|-----------|
| 1. Orient (방향 설정) | **A** | 도메인 분리 명확, 타입 정의 체계적 |
| 2. Simplify (단순화) | **B+** | 전반적으로 단순하나 일부 중복 패턴 존재 |
| 3. Cut Surgically (정밀 절제) | **B** | 불필요한 코드 일부 존재 (미사용 변수, 중복 매핑) |
| 4. Preserve Guarantees (보장 유지) | **B+** | DB 제약조건 양호하나 API 검증 일부 누락 |
| 5. Verify (검증) | **B-** | 에러 핸들링 기본적, 테스트 코드 없음 |
| 6. Report Honestly (정직한 보고) | **A** | 상태 관리 투명, 사용자 피드백 명확 |

---

## 원칙별 상세 분석

### 1. Orient / 방향 설정 — 등급: **A**

> *"문제를 정확히 이해하고, 올바른 방향을 설정했는가?"*

**긍정적 근거:**

- **도메인 분리가 명확**합니다. `types.ts`에 4개 도메인(수합, 월중계획, 메일머지, 구입요청)의 타입이 깔끔하게 분리되어 있습니다.
- **API 라우트 구조가 RESTful**하고 직관적입니다. `/api/runs`, `/api/submissions`, `/api/plans`, `/api/entries`, `/api/templates`, `/api/purchase/items`, `/api/purchase/requests`, `/api/mailmerge/templates`, `/api/mailmerge/runs` — 각 엔드포인트의 역할이 명확합니다.
- **DB 스키마 설계가 도메인을 잘 반영**합니다. `CHECK` 제약조건, `UNIQUE` 제약조건, `ON DELETE CASCADE` 등이 적절히 사용되었습니다.
- **페이지 컴포넌트의 역할 분리**가 잘 되어 있습니다. 입력(`input`), 조회(`page`), 검토(`review`), 결과(`results`)가 명확히 분리됩니다.
- **서버/클라이언트 컴포넌트 분리**가 적절합니다. 데이터 페치가 필요한 곳은 서버 컴포넌트, 인터랙션이 필요한 곳은 `'use client'`로 명시.

**개선 가능한 부분:**

- `page.tsx` (대시보드)에서 `getRuns()` 함수가 `status=open`으로만 조회하지만, 마감된 수합도 함께 보여주기 위해 별도 로직이 필요합니다. 현재는 `closed` 상태를 별도로 페치하지 않아 `closedRuns` 필터가 클라이언트에서만 동작합니다. (서버에서 이미 필터링된 데이터를 받으므로 실제로는 동작하지만, 구조가 직관적이지 않습니다.)

---

### 2. Simplify / 단순화 — 등급: **B+**

> *"불필요한 복잡성을 제거하고 가장 단순한 해법을 선택했는가?"*

**긍정적 근거:**

- **API 라우트가 단순합니다.** 각 라우트는 GET/POST/PATCH/DELETE를 하나의 파일에서 처리하는 패턴으로 일관되어 있습니다.
- **상태 관리가 단순합니다.** 별도 상태 관리 라이브러리(Zustand, Redux 등) 없이 React 기본 `useState`만 사용합니다. 프로젝트 규모에 적절합니다.
- **타입 정의가 간결합니다.** `types.ts` 256줄에 모든 도메인 타입이 정의되어 있고, 불필요한 제네릭이나 복잡한 타입 연산이 없습니다.
- **Supabase 클라이언트 설정이 단순합니다.** `supabase.ts` 15줄로 클라이언트와 서비스 롤 클라이언트를 깔끔하게 분리했습니다.

**개선 가능한 부분:**

- **CamelCase ↔ Snake_case 매핑이 반복적으로 등장합니다.** `mailmerge/runs/route.ts`의 `fieldMap` (12개 필드), `mailmerge/templates/route.ts`의 `fieldMap` (8개 필드), 그리고 서버 컴포넌트에서 수동으로 매핑하는 코드가 여러 곳에 산재합니다. 이를 유틸리티 함수로 통합할 수 있습니다.

  ```typescript
  // 현재: 매 파일에서 반복
  const fieldMap: Record<string, string> = {
    title: 'title',
    grade: 'grade',
    classNum: 'class_num',
    // ... 매번 작성
  }

  // 개선 제안: 공통 유틸리티
  function toSnake(obj: Record<string, any>, mapping: Record<string, string>) { ... }
  function toCamel(obj: any, mapping: Record<string, string>) { ... }
  ```

- **`collect/[id]/page.tsx`와 `collect/[id]/results/page.tsx`** 모두 `RunDetail` 인터페이스를 별도로 정의하고 있습니다. 이를 `types.ts`에서 공유하면 중복을 줄일 수 있습니다.

---

### 3. Cut Surgically / 정밀 절제 — 등급: **B**

> *"불필요한 코드를 정확히 제거했는가? 기능 비율 대비 코드량이 적절한가?"*

**긍정적 근거:**

- **기능 대비 코드량이 효율적입니다.** 4개 도메인(수합, 월중계획, 메일머지, 구입요청)의 CRUD를 약 3,500줄로 구현했습니다. 도메인당 약 875줄로 양호한 수준입니다.
- **불필요한 추상화가 없습니다.** 과도한 디자인 패턴이나 인터페이스 계층 없이 직접적인 구현을 따릅니다.
- **CSS-in-JS 스타일링이 일관됩니다.** Tailwind CSS 클래스와 인라인 스타일을 혼용하지만, 각 컴포넌트 내에서는 일관된 패턴을 유지합니다.

**개선 가능한 부분:**

- **미사용 변수/필드가 존재합니다:**
  - `collect_submissions.submitter_id` 컬럼이 스키마에 정의되어 있지만 코드 어디에서도 사용되지 않습니다.
  - `MonthlyPlanEntry` 타입에 `weekNumber`와 `dayOfWeek` 필드가 있지만, DB에서 `GENERATED ALWAYS AS`로 자동 생성되는 컬럼입니다. 클라이언트에서 별도로 전송하지 않도록 주의가 필요합니다.
  - `purchase/requests/route.ts`의 GET에서 `grade`와 `classNum`이 필수 검증(`!itemId || !grade`)에 포함되지만, 실제로는 선택적 조회가 더 자연스럽습니다.

- **중복된 타입 매핑 로직:**
  - `mailmerge/page.tsx`의 `getTemplates()`에서 `document_type → documentType` 등 매핑
  - `mailmerge/page.tsx`의 `getRecentRuns()`에서 동일 패턴 매핑
  - `mailmerge/runs/[id]/page.tsx`의 `getRun()`에서 동일 패턴 매핑
  - `mailmerge/templates/[id]/page.tsx`의 `getTemplate()`에서 동일 패턴 매핑
  - 이 4곳의 매핑 로직은 동일한 패턴이며, 유틸리티로 통합하면 약 60~80줄을 줄일 수 있습니다.

- **`admin/new/page.tsx`의 `targetValue` 처리 로직** (74-76줄)이 다소 복잡합니다. `JSON.stringify(targetVal)`을 한 번 더 감싸는 구조(`targetValue: targetVal ? JSON.stringify(targetVal) : null`)는 의도가 불분명합니다.

---

### 4. Preserve Guarantees / 보장 유지 — 등급: **B+**

> *"기존 기능을 깨뜨리지 않고, 데이터 무결성과 안전성을 보장하는가?"*

**긍정적 근거:**

- **DB 레벨 제약조건이 견고합니다:**
  - `CHECK` 제약조건으로 상태 값 도메인을 제한 (`status IN (...)`)
  - `UNIQUE(run_id, grade, class_num)`으로 중복 제출 방지
  - `UNIQUE(item_id, grade, class_num)`으로 구입요청 중복 방지
  - `ON DELETE CASCADE`로 참조 무결성 유지
  - `NOT NULL` 제약조건으로 필수 필드 보장

- **API에서 기본적인 입력 검증이 있습니다:**
  - 필수 파라미터 체크 (`planId required`, `runId required`, `id required`)
  - 중복 제출 방지 로직 (`submissions/route.ts`의 `existing` 체크)
  - Upsert 패턴 (`purchase/requests/route.ts`의 기존 신청 확인 후 업데이트)

- **상태 전환이 안전합니다.** `PATCH` 메서드로 상태 변경 시 `id`와 `status`를 필수로 요청합니다.

**개선 가능한 부분:**

- **API 검증이 일관되지 않습니다:**
  - `plans/route.ts` POST에서는 `year`, `month` 필수 검증이 없습니다. `month` 없이 생성 가능합니다.
  - `runs/route.ts` POST에서는 `deadline` 필수 검증이 없습니다.
  - `entries/route.ts` POST의 단일 엔트리 경우 `planId`, `planDate`, `content` 필수 검증이 없습니다.
  - `templates/route.ts` POST에서는 `title`만 검증하고 `fields` 내부 검증이 없습니다.

- **에러 메시지가 일관되지 않습니다:**
  - 일부는 한국어 (`'물품명은 필수입니다'`), 일부는 영어 (`'planId required'`)
  - 일부는 에러 객체를 그대로 전달 (`err.message`), 일부는 커스텀 메시지

- **트랜잭션 처리가 없습니다.** `submissions/route.ts`에서 제출 업데이트 시 기존 답변 삭제(`delete`) 후 새 답변 삭제(`insert`)가 별도 쿼리로 실행됩니다. 실패 시 데이터 불일치가 발생할 수 있습니다.

---

### 5. Verify / 검증 — 등급: **B-**

> *"코드가 올바르게 동작하는지 검증할 수 있는가? 테스트와 에러 핸들링이 충분한가?"*

**긍정적 근거:**

- **에러 핸들링의 기본 구조가 있습니다.** 모든 API 라우트에서 `try-catch` 또는 `.catch()` 패턴을 사용하고, 에러 발생 시 적절한 HTTP 상태 코드(400, 500)를 반환합니다.
- **클라이언트에서 사용자 피드백을 제공합니다.** `alert()` 및 상태 메시지(`formMsg`, `message`, `error`)를 통해 결과를 알립니다.
- **CSV 다운로드 기능**이 여러 페이지에 구현되어 있어 데이터 검증이 가능합니다.

**개선 가능한 부분:**

- **테스트 코드가 전혀 없습니다.** 단위 테스트, 통합 테스트, E2E 테스트 어디에도 해당하지 않습니다. 이는 프로젝트 규모가 아직 초기 단계(0.1.0)이므로 이해되나, 핵심 비즈니스 로직(중복 제출 방지, 상태 전환)에 대한 테스트가 있으면 좋겠습니다.

- **에러 핸들링이 기본적입니다:**
  - `alert()`에 의존하는 클라이언트 에러 처리는 사용자 경험이 좋지 않습니다. Toast 알림(`review/page.tsx`에 구현된 것처럼)으로 통합하면 좋겠습니다.
  - API에서 `catch` 블록이 없는 곳이 있습니다. (`entries/route.ts` DELETE 등)

- **타입 안전성 검증이 부족합니다:**
  - `any` 타입이 여러 곳에서 사용됩니다. (`body: any`, `updates: any`, `e: any`)
  - `JSON.parse()` 호출 시 에러 핸들링이 없습니다. (`targetLabel` 함수, `select` 필드 옵션 파싱)
  - `new Date(e.target.value)` 같은 사용자 입력 기반 Date 생성은 유효하지 않은 값에 대해 `Invalid Date`를 반환할 수 있습니다.

- **로딩 상태 관리가 일관되지 않습니다.** 일부 컴포넌트는 `loading` 상태를 가지지만, 일부는 초기 로딩 후 데이터 리프레시 시 로딩 표시가 없습니다.

---

### 6. Report Honestly / 정직한 보고 — 등급: **A**

> *"시스템의 상태를 정확하고 투명하게 보고하는가? 사용자에게 정직한 피드백을 주는가?"*

**긍정적 근거:**

- **상태 관리가 투명합니다.** 각 엔티티의 상태(`open/closed/archived`, `draft/submitted/confirmed/rejected` 등)가 명확하게 정의되어 있고, 한국어 라벨 매핑이 일관되게 제공됩니다.
- **사용자 피드백이 명확합니다.** 제출 완료, 임시저장, 마감 지남, 충돌 감지 등 다양한 상황에 대한 피드백이 있습니다.
- **진행률 표시가 있습니다.** 결과 페이지에서 제출 현황 프로그레스 바, 검토 페이지에서 상태별 카운트를 보여줍니다.
- **에러 상황을 숨기지 않습니다.** `errorMessage` 필드가 템플릿에 노출되고, 실패 상태가 UI에 명확히 표시됩니다.
- **메일머지 실행 가이드**가 상태별로 상세하게 제공됩니다. (`runs/[id]/page.tsx`의 `statusStyle`)

**개선 가능한 부분:**

- **낙관적 업데이트(Optimistic Update)가 없습니다.** 모든 작업 후 `fetch`로 재조회하거나 `alert`로 완료를 알립니다. 소규모 내부 도구이므로 현재 수준은 충분하지만, 사용자 경험 개선 여지가 있습니다.

---

## 📈 개선 제안 (우선순위별)

### 🔴 높음 (정확성/안전성 관련)

| # | 제안 | 영향 | 예상 공수 |
|---|------|------|-----------|
| 1 | **API 입력 검증 강화** — `plans`, `runs`, `entries` POST에서 필수 필드 검증 추가 | 데이터 무결성 | 0.5일 |
| 2 | **JSON.parse() 에러 핸들링** — `targetLabel`, select 옵션 파싱 시 `try-catch` 추가 | 런타임 크래시 방지 | 0.5일 |
| 3 | **트랜잭션 처리** — `submissions` 업데이트 시 답변 삭제+삽입을 트랜잭션으로 묶기 | 데이터 일관성 | 1일 |

### 🟡 중간 (유지보수성/품질 관련)

| # | 제안 | 영향 | 예상 공수 |
|---|------|------|-----------|
| 4 | **CamelCase ↔ Snake_case 매핑 유틸리티화** — 4개 파일의 중복 매핑 로통합 | 코드 중복 제거 ~80줄 | 1일 |
| 5 | **에러 메시지 언어 통일** — 한국어 또는 영어로 일관되게 통일 | 사용자 경험 | 0.5일 |
| 6 | **`any` 타입 제거** — API 라우트의 body 타입을 명시적으로 정의 | 타입 안전성 | 1일 |
| 7 | **미사용 필드 정리** — `submitter_id`, `targetValue` 이중 직렬화 등 | 코드 명확성 | 0.5일 |

### 🟢 낮음 (경험 개선 관련)

| # | 제안 | 영향 | 예상 공수 |
|---|------|------|-----------|
| 8 | **Toast 알림 통합** — `alert()`를 커스텀 Toast 컴포넌트로 대체 | UX 개선 | 1일 |
| 9 | **공통 `RunDetail` 타입 추출** — `collect/[id]`와 `collect/[id]/results`의 중복 인터페이스 통합 | 타입 재사용 | 0.5일 |
| 10 | **기본 테스트 추가** — 핵심 API 라우트에 대한 통합 테스트 | 회귀 방지 | 2일 |

---

## 📝 총평

**school-collect**는 학교 업무 자동화라는 명확한 문제 정의 하에, Next.js + Supabase 스택으로 효율적으로 구현된 프로젝트입니다.

**강점:**
- 도메인 분리와 타입 설계가 체계적
- DB 스키마의 제약조견 설계가 견고
- 코드가 전반적으로 읽기 쉽고 단순
- 상태 관리와 사용자 피드백이 투명

**약점:**
- API 입력 검증의 일관성 부족
- CamelCase ↔ Snake_case 매핑 중복
- 테스트 코드 부재
- `any` 타입 사용으로 인한 타입 안전성 저하

**종합적으로**, 이 프로젝트는 Karpathy의 6원칙 관점에서 **"잘 방향이 잡히고, 단순하며, 정직한"** 코드입니다. 개선 제안 사항들은 대부분 소규모 리팩토링으로 해결 가능하며, 우선순위대로 적용하면 **A 등급**으로의 도달이 충분히 가능합니다.

---

*본 리포트는 OWL에 의해 자동 생성되었습니다.*
