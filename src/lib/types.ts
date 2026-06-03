// ─── 타입 정의 ───

export interface FormTemplate {
  id: string
  title: string
  description: string | null
  year: number
  createdAt: string
  updatedAt: string
  fields?: FormField[]
}

export interface FormField {
  id: string
  templateId: string
  label: string
  fieldType: 'text' | 'number' | 'date' | 'select' | 'textarea'
  required: boolean
  options: string | null // JSON array
  order: number
}

export interface CollectRun {
  id: string
  templateId: string
  year: number
  deadline: string
  status: 'open' | 'closed' | 'archived'
  targetType: 'all' | 'grade' | 'class' | 'custom'
  targetValue: string | null
  description: string | null
  createdAt: string
  updatedAt: string
  template?: FormTemplate
  submissionCount?: number
  totalTargets?: number
}

export interface CollectSubmission {
  id: string
  runId: string
  grade: number | null
  classNum: number | null
  submitter: string | null
  submittedAt: string
  updatedAt: string
  answers?: CollectAnswer[]
}

export interface CollectAnswer {
  id: string
  submissionId: string
  fieldId: string
  value: string | null
}


// ─── 월중계획 타입 ───

export interface MonthlyPlan {
  id: string
  year: number
  month: number
  title: string
  description: string | null
  deadline: string | null
  status: 'open' | 'collecting' | 'reviewing' | 'published' | 'archived'
  createdAt: string
  updatedAt: string
  entries?: MonthlyPlanEntry[]
  entryCount?: number
  confirmedCount?: number
  submittedCount?: number
}

export interface MonthlyPlanEntry {
  id: string
  planId: string
  planDate: string
  content: string
  target: string
  location: string
  personInCharge: string
  notes: string
  submitter: string
  weekNumber: number
  dayOfWeek: number
  status: 'draft' | 'submitted' | 'confirmed' | 'rejected'
  createdAt: string
  updatedAt: string
}

export function planStatusLabel(s: string): string {
  const map: Record<string, string> = {
    open: '준비중',
    collecting: '수합중',
    reviewing: '검토중',
    published: '확정공개',
    archived: '보관',
  }
  return map[s] || s
}

export function entryStatusLabel(s: string): string {
  const map: Record<string, string> = {
    draft: '작성중',
    submitted: '제출',
    confirmed: '확정',
    rejected: '반려',
  }
  return map[s] || s
}

export function monthLabel(m: number): string {
  return `${m}월`
}

export function dayOfWeekLabel(d: number, short = false): string {
  const map = short
    ? ['일', '월', '화', '수', '목', '금', '토']
    : ['일요일', '월요일', '화요일', '수요일', '목요일', '금요일', '토요일']
  return map[d] || ''
}


// 헬퍼
export function fieldTypeLabel(t: string): string {
  const map: Record<string, string> = {
    text: '텍스트', number: '숫자', date: '날짜',
    select: '선택', textarea: '긴텍스트',
  }
  return map[t] || t
}

export function statusBadgeClass(s: string): string {
  const map: Record<string, string> = { open: 'badge-open', closed: 'badge-closed', archived: 'badge-archived' }
  return map[s] || ''
}

export function statusLabel(s: string): string {
  const map: Record<string, string> = { open: '진행중', closed: '마감', archived: '보관' }
  return map[s] || s
}

/** 등급/반 값으로 대상 표시 문자열 생성 */
export function targetLabel(run: { targetType: string; targetValue: string | null }): string {
  if (run.targetType === 'all') return '전체'
  if (run.targetType === 'grade' && run.targetValue) {
    const grades = JSON.parse(run.targetValue)
    return `${grades.join(',')}학년`
  }
  if (run.targetType === 'class' && run.targetValue) {
    const classes = JSON.parse(run.targetValue)
    return `${classes.join(', ')}`
  }
  return run.targetType
}


// ─── 메일머지 타입 ───

export interface MailmergeTemplate {
  id: string
  title: string
  description: string | null
  documentType: string
  year: number
  fields: MailmergeField[]
  templateFilePath: string | null
  isActive: boolean
  createdAt: string
  updatedAt: string
  runs?: MailmergeRun[]
}

export interface MailmergeField {
  name: string
  label: string
  type: 'text' | 'number' | 'select'
  options?: string[]
}

export interface MailmergeRun {
  id: string
  templateId: string
  title: string
  grade: number | null
  classNum: number | null
  studentCount: number
  dataFilePath: string | null
  status: 'draft' | 'data_ready' | 'processing' | 'completed' | 'failed'
  corrections: string | null
  outputPath: string | null
  errorMessage: string | null
  createdAt: string
  updatedAt: string
  template?: MailmergeTemplate
}

export function mailmergeStatusLabel(s: string): string {
  const map: Record<string, string> = {
    draft: '준비중',
    data_ready: '데이터 준비',
    processing: '생성중',
    completed: '완료',
    failed: '실패',
  }
  return map[s] || s
}

export function docTypeLabel(s: string): string {
  const map: Record<string, string> = {
    general: '일반',
    health_check: '신체검사',
    dental: '구강검진',
    urine: '소변검사',
    vision: '시력검사',
    grade_report: '성적통지표',
    consent: '동의서',
    notice: '안내장',
    etc: '기타',
  }
  return map[s] || s
}


// ─── 정보화기기 구입요청 타입 ───

export interface PurchaseItem {
  id: string
  name: string
  spec: string
  unitPrice: number
  unit: string
  link: string
  description: string
  category: string
  isActive: boolean
  createdAt: string
  updatedAt: string
  requestCount?: number
  totalQuantity?: number
  totalAmount?: number
}

export interface PurchaseRequest {
  id: string
  itemId: string
  grade: number
  classNum: number
  quantity: number
  submitter: string
  notes: string
  createdAt: string
  item?: PurchaseItem
}


// ─── 개인정보파일 연간 정비 시스템 타입 ───

export interface PrivacyFileStandard {
  id: string
  businessArea: string
  fileName: string
  retentionPeriod: string
  department: string
  guide: string | null
  isActive: boolean
  createdAt: string
}

export interface PrivacyMaintenanceCycle {
  id: string
  year: number
  startDate: string
  endDate: string
  status: 'draft' | 'active' | 'completed'
  description: string | null
  createdAt: string
  updatedAt: string
  totalDepartments?: number
  submittedDepartments?: number
  entryCount?: number
  submittedCount?: number
}

export interface PrivacyDepartment {
  id: string
  name: string
  displayOrder: number
  isActive: boolean
  createdAt: string
  currentManager?: PrivacyDepartmentManager
  files?: PrivacyFileStandard[]
  entries?: PrivacyMaintenanceEntry[]
}

export interface PrivacyDepartmentManager {
  id: string
  departmentId: string
  name: string
  position: string | null
  phone: string | null
  email: string | null
  isActive: boolean
  createdAt: string
  updatedAt: string
}

export interface PrivacyMaintenanceEntry {
  id: string
  cycleId: string
  departmentId: string
  standardId: string
  dataSubjectCount: number | null
  hasAnomaly: boolean
  anomalyDescription: string | null
  managerName: string | null
  managerPosition: string | null
  notes: string | null
  isSubmitted: boolean
  submittedAt: string | null
  createdAt: string
  updatedAt: string
  department?: PrivacyDepartment
  standard?: PrivacyFileStandard
}

export interface PrivacyMaintenanceHistory {
  id: string
  cycleId: string
  year: number
  exportData: ExportData[]
  submittedBy: string | null
  submittedAt: string | null
  createdAt: string
}

export interface ExportData {
  fileName: string
  businessArea: string
  dataSubjectCount: number
  retentionPeriod: string
  hasAnomaly: boolean
  anomalyDescription: string | null
  department: string
  managerName: string | null
  notes: string | null
}

export interface DashboardSummary {
  cycle: PrivacyMaintenanceCycle
  totalDepartments: number
  completedDepartments: number
  totalEntries: number
  submittedEntries: number
  pendingEntries: number
  departmentProgress: DepartmentProgress[]
  recentHistory: PrivacyMaintenanceHistory[]
}

export interface DepartmentProgress {
  department: PrivacyDepartment
  totalFiles: number
  submittedFiles: number
  pendingFiles: number
  entries: PrivacyMaintenanceEntry[]
}

export function cycleStatusLabel(status: string): string {
  const map: Record<string, string> = {
    draft: '초안',
    active: '진행중',
    completed: '완료',
  }
  return map[status] || status
}

export function calculateProgress(submitted: number, total: number): number {
  if (total === 0) return 0
  return Math.round((submitted / total) * 100)
}


// ─── 권한/인증 시스템 타입 ───

export type TeacherRole = 'admin' | 'department_head' | 'teacher'

export interface ScSchool {
  id: string
  name: string
  eduOfficeCode: string | null
  schoolCode: string | null
  isActive: boolean
  createdAt: string
}

export interface ScTeacher {
  id: string
  authUserId: string
  schoolId: string
  name: string
  email: string
  phone: string | null
  role: TeacherRole
  isActive: boolean
  createdAt: string
  updatedAt: string
  school?: ScSchool
  departments?: string[]  // department names
}

export interface ScTeacherDept {
  id: string
  teacherId: string
  departmentName: string
  isPrimary: boolean
}

export interface AuthSession {
  user: {
    id: string
    email: string
  }
  teacher: ScTeacher
}

export function roleLabel(role: TeacherRole): string {
  const map: Record<TeacherRole, string> = {
    admin: '관리자',
    department_head: '부서장',
    teacher: '교사',
  }
  return map[role]
}


// ─── 카드 지출 관리 타입 ───

export interface CardExpenseCard {
  id: string
  cardName: string
  cardNumber: string
  cardHolder: string
  issuingBank: string
  isActive: boolean
  createdAt: string
  updatedAt: string
  currentHolder?: string  // join: 현재 수령자
  expenseCount?: number
  totalAmount?: number
}

export interface CardAssignment {
  id: string
  cardId: string
  teacherName: string
  takenAt: string
  returnedAt: string | null
  note: string
  createdAt: string
  card?: CardExpenseCard
}

export interface CardExpense {
  id: string
  cardId: string
  assignmentId: string | null
  teacherName: string
  amount: number
  merchant: string
  merchantCategory: string
  expenseDate: string
  receiptImageUrl: string
  memo: string
  source: 'manual' | 'sms_webhook' | 'card_statement'
  createdAt: string
  updatedAt: string
  card?: CardExpenseCard
  assignment?: CardAssignment
}

export function expenseSourceLabel(s: string): string {
  const map: Record<string, string> = {
    manual: '수동',
    sms_webhook: '문자',
    card_statement: '명세서',
  }
  return map[s] || s
}


// ─── 출장여비 신청 타입 ───

export interface TripDistanceStd {
  id: string
  region: string
  baseDistance: number
  roundTrip: boolean
  createdAt: string
}

export interface TripExpenseRequest {
  id: string
  teacherName: string
  department: string
  destination: string
  purpose: string
  transport: 'car' | 'bus' | 'train' | 'etc'
  tripDate: string
  distance: number
  fuelUnitPrice: number
  fuelEfficiency: number
  fuelCost: number
  tollCost: number
  parkingCost: number
  otherCost: number
  totalCost: number
  receiptImageUrl: string
  note: string
  status: 'draft' | 'submitted' | 'approved' | 'rejected'
  submittedAt: string | null
  reviewedBy: string
  reviewNote: string
  createdAt: string
  updatedAt: string
}

export function tripStatusLabel(s: string): string {
  const map: Record<string, string> = {
    draft: '작성중',
    submitted: '제출',
    approved: '승인',
    rejected: '반려',
  }
  return map[s] || s
}

export function transportLabel(s: string): string {
  const map: Record<string, string> = {
    car: '자가용',
    bus: '버스',
    train: '기차',
    etc: '기타',
  }
  return map[s] || s
}

export function fmtMoney(n: number): string {
  return n.toLocaleString('ko-KR') + '원'
}



// ─── 교육과정 작성 모듈 타입 (v2) ───

export interface CurriculumYear {
  id: string
  schoolId: string
  year: number
  status: 'draft' | 'editing' | 'validated' | 'confirmed' | 'archived'
  isActive: boolean
  createdAt: string
  updatedAt: string
}

export interface CurriculumClass {
  id: string
  curriculumYearId: string
  grade: number
  classNo: number
  homeroomTeacherId: string | null
  studentCount: number
  createdAt: string
  updatedAt: string
}

export interface TeacherSubjectAssignment {
  id: string
  curriculumYearId: string
  teacherId: string
  subjectId: string
  grade: number | null
  classNo: number | null
  createdAt: string
}

export interface SchoolYear {
  id: string
  year: number
  term: number
  active: boolean
  schoolId: string | null
  createdAt: string
}

export interface Subject {
  id: string
  code: string
  name: string
  category: 'general' | 'creative' | 'discretionary' | 'elective'
  createdAt: string
}

export interface CurriculumHour {
  id: string
  yearId: string
  grade: number
  subjectId: string
  totalHours: number
  term1Hours: number | null
  term2Hours: number | null
  requiredHours: number | null
  createdAt: string
  updatedAt: string
}

export interface CalendarEvent {
  id: string
  yearId: string
  date: string
  type: 'holiday' | 'event' | 'break' | 'exam' | 'etc'
  title: string
  description: string | null
  isSchoolDay: boolean
  affectsTimetable: boolean
  affectedPeriods: number[] | null
  gradeScope: number[] | null
  createdAt: string
  updatedAt: string
}

export interface TimetableEntry {
  id: string
  yearId: string
  sourceType: 'base' | 'semester'
  term: number | null
  grade: number
  classNo: number
  weekday: number
  period: number
  subjectId: string | null
  teacherId: string | null
  createdAt: string
  updatedAt: string
}

export interface Lesson {
  id: string
  yearId: string
  term: number
  grade: number
  classNo: number
  subjectId: string
  unit: string
  lessonNo: number
  content: string
  startDate: string | null
  endDate: string | null
  period: number | null
  createdAt: string
  updatedAt: string
}

export interface CurriculumProject {
  id: string
  yearId: string
  term: number
  grade: number
  classNo: number
  title: string
  description: string | null
  startDate: string | null
  endDate: string | null
  createdAt: string
  updatedAt: string
  subjects?: CurriculumProjectSubject[]
}

export interface CurriculumProjectSubject {
  id: string
  projectId: string
  subjectId: string
  lessonId: string | null
  createdAt: string
  subject?: Subject
  lesson?: Lesson
}

export interface CrossTopic {
  id: string
  yearId: string
  name: string
  category: string
  defaultHours: number
  createdAt: string
}

export interface CrossTopicLog {
  id: string
  yearId: string
  lessonId: string
  topicId: string
  hours: number
  note: string | null
  createdAt: string
  topic?: CrossTopic
  lesson?: Lesson
}

export interface ElectiveSubject {
  id: string
  yearId: string
  name: string
  grade: number | null
  totalHours: number
  subjectId: string | null
  createdAt: string
  updatedAt: string
}

export interface ElectiveTimetable {
  id: string
  electiveId: string
  grade: number
  classNo: number
  weekday: number
  period: number
  teacherId: string | null
  createdAt: string
}
