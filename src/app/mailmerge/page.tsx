import Link from 'next/link'
import { getServiceSupabase } from '@/lib/supabase'
import { mailmergeStatusLabel, docTypeLabel } from '@/lib/types'
import type { MailmergeTemplate, MailmergeRun } from '@/lib/types'

function statusTagClass(status: string) {
  if (status === 'completed') return 'green'
  if (status === 'processing') return 'blue'
  if (status === 'failed') return 'red'
  if (status === 'data_ready') return 'amber'
  return 'gray'
}

async function getTemplates(): Promise<MailmergeTemplate[]> {
  const sb = getServiceSupabase()
  const { data } = await sb.from('mailmerge_templates')
    .select('*')
    .eq('is_active', true)
    .order('year', { ascending: false })
    .order('created_at', { ascending: false })
  return (data || []).map((t: any) => ({
    ...t,
    documentType: t.document_type,
    templateFilePath: t.template_file_path,
    isActive: t.is_active,
    createdAt: t.created_at,
    updatedAt: t.updated_at,
  }))
}

async function getRecentRuns(): Promise<MailmergeRun[]> {
  const sb = getServiceSupabase()
  const { data } = await sb.from('mailmerge_runs')
    .select('*, template:mailmerge_templates(*)')
    .order('created_at', { ascending: false })
    .limit(10)
  return (data || []).map((r: any) => ({
    ...r,
    templateId: r.template_id,
    classNum: r.class_num,
    studentCount: r.student_count,
    dataFilePath: r.data_file_path,
    outputPath: r.output_path,
    errorMessage: r.error_message,
    createdAt: r.created_at,
    updatedAt: r.updated_at,
  }))
}

export default async function MailMergeDashboard() {
  const [templates, recentRuns] = await Promise.all([getTemplates(), getRecentRuns()])

  const byType: Record<string, MailmergeTemplate[]> = {}
  for (const t of templates) {
    const key = t.documentType || 'general'
    if (!byType[key]) byType[key] = []
    byType[key].push(t)
  }

  const completedCount = recentRuns.filter(r => r.status === 'completed').length
  const pendingCount = recentRuns.filter(r => r.status === 'draft' || r.status === 'data_ready').length

  return (
    <div className="max-w-5xl">
      {/* 상단 — 문서 헤더 스타일 */}
      <div className="doc p-5 mb-6">
        <div className="flex items-center justify-between">
          <div>
            <div className="sub mb-1">📄 가정통신문 메일머지</div>
            <h1 className="text-xl font-extrabold tracking-tight" style={{ color: 'var(--ink)' }}>메일머지 관리</h1>
          </div>
          <Link href="/mailmerge/runs/new" className="doc-btn primary">
            + 새 메일머지 실행
          </Link>
        </div>
      </div>

      {/* 통계 */}
      <div className="stat-grid">
        <div className="stat-box">
          <div className="num">{templates.length}</div>
          <div className="label">등록된 템플릿</div>
        </div>
        <div className="stat-box">
          <div className="num" style={{ color: 'var(--green)' }}>{completedCount}</div>
          <div className="label">완료</div>
        </div>
        <div className="stat-box">
          <div className="num" style={{ color: '#8b6d00' }}>{pendingCount}</div>
          <div className="label">준비중</div>
        </div>
        <div className="stat-box">
          <div className="num">{recentRuns.length}</div>
          <div className="label">전체 실행</div>
        </div>
      </div>

      {/* 템플릿 목록 */}
      <div className="doc p-5 mb-6">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-base font-bold" style={{ color: 'var(--ink)' }}>템플릿</h2>
          <Link href="/mailmerge/templates/new" className="doc-link">+ 새 템플릿 등록</Link>
        </div>

        {templates.length === 0 ? (
          <div className="doc-empty">
            등록된 템플릿이 없습니다.<br />
            <Link href="/mailmerge/templates/new" className="doc-link mt-2 inline-block">첫 템플릿 등록하기</Link>
          </div>
        ) : (
          <table className="doc-table">
            <thead>
              <tr>
                <th>문서 유형</th>
                <th>템플릿명</th>
                <th>연도</th>
                <th>필드</th>
                <th>파일</th>
              </tr>
            </thead>
            <tbody>
              {Object.entries(byType).map(([type, tpls]) =>
                tpls.map((t, i) => (
                  <tr key={t.id}>
                    {i === 0 && (
                      <td rowSpan={tpls.length} className="font-bold text-xs" style={{ color: 'var(--muted)' }}>
                        {docTypeLabel(type)}
                      </td>
                    )}
                    <td><Link href={`/mailmerge/templates/${t.id}`} className="doc-link">{t.title}</Link></td>
                    <td className="text-xs" style={{ color: 'var(--muted)' }}>{t.year}년</td>
                    <td><span className="doc-tag gray">{t.fields?.length || 0}개</span></td>
                    <td className="text-xs" style={{ color: 'var(--muted)' }}>{t.templateFilePath ? '📎 있음' : '—'}</td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        )}
      </div>

      {/* 최근 실행 기록 */}
      <div className="doc p-5">
        <div className="section-head mb-3">
          <h2>최근 실행 기록</h2>
          <span className="meta">최근 10건</span>
        </div>

        {recentRuns.length === 0 ? (
          <div className="doc-empty">실행 기록이 없습니다.</div>
        ) : (
          <table className="doc-table">
            <thead>
              <tr>
                <th>실행명</th>
                <th>템플릿</th>
                <th>대상</th>
                <th>인원</th>
                <th>일시</th>
                <th>상태</th>
              </tr>
            </thead>
            <tbody>
              {recentRuns.map(run => (
                <tr key={run.id}>
                  <td><Link href={`/mailmerge/runs/${run.id}`} className="doc-link">{run.title}</Link></td>
                  <td className="text-xs" style={{ color: 'var(--muted)' }}>{run.template?.title || '—'}</td>
                  <td className="text-xs" style={{ color: 'var(--muted)' }}>
                    {run.grade ? `${run.grade}학년 ` : ''}{run.classNum ? `${run.classNum}반` : ''}
                  </td>
                  <td className="text-xs">{run.studentCount}명</td>
                  <td className="text-xs" style={{ color: 'var(--muted)' }}>
                    {new Date(run.createdAt).toLocaleDateString('ko-KR')}
                  </td>
                  <td>
                    <span className={`doc-tag ${statusTagClass(run.status)}`}>
                      {mailmergeStatusLabel(run.status)}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  )
}
