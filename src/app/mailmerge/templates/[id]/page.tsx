import Link from 'next/link'
import { getServiceSupabase } from '@/lib/supabase'
import { mailmergeStatusLabel, docTypeLabel } from '@/lib/types'
import type { MailmergeTemplate, MailmergeRun } from '@/lib/types'

async function getTemplate(id: string): Promise<MailmergeTemplate | null> {
  const sb = getServiceSupabase()
  const { data } = await sb.from('mailmerge_templates')
    .select('*, runs:mailmerge_runs(*)')
    .eq('id', id)
    .single()
  if (!data) return null
  return {
    ...data,
    documentType: data.document_type,
    templateFilePath: data.template_file_path,
    isActive: data.is_active,
    createdAt: data.created_at,
    updatedAt: data.updated_at,
    runs: (data.runs || []).map((r: any) => ({
      ...r,
      templateId: r.template_id,
      classNum: r.class_num,
      studentCount: r.student_count,
      dataFilePath: r.data_file_path,
      outputPath: r.output_path,
      errorMessage: r.error_message,
      createdAt: r.created_at,
      updatedAt: r.updated_at,
    })),
  }
}

export default async function TemplateDetailPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params
  const template = await getTemplate(id)

  if (!template) {
    return (
      <div className="doc-empty" style={{ paddingTop: 80 }}>
        <div className="text-base mb-2">템플릿을 찾을 수 없습니다</div>
        <Link href="/mailmerge" className="doc-link">← 메일머지로 돌아가기</Link>
      </div>
    )
  }

  const runs = (template.runs || []).sort((a, b) =>
    new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime()
  )

  return (
    <div className="max-w-4xl">
      {/* 상단 네비 */}
      <Link href="/mailmerge" className="doc-link text-xs mb-4 inline-block">← 메일머지</Link>

      {/* 문서 헤더 */}
      <div className="doc p-5 mb-5">
        <div className="flex items-start justify-between">
          <div>
            <div className="sub mb-1">{docTypeLabel(template.documentType)}</div>
            <h1 className="text-xl font-extrabold tracking-tight" style={{ color: 'var(--ink)' }}>{template.title}</h1>
            {template.description && (
              <p className="text-sm mt-2" style={{ color: 'var(--muted)' }}>{template.description}</p>
            )}
          </div>
          <Link href={`/mailmerge/runs/new?template=${template.id}`} className="doc-btn primary">
            + 이 템플릿으로 실행
          </Link>
        </div>
        {template.templateFilePath && (
          <div className="mt-3 text-xs" style={{ color: 'var(--muted)' }}>
            📎 템플릿 파일: {template.templateFilePath}
          </div>
        )}
      </div>

      {/* 템플릿 기본 정보 */}
      <div className="stat-grid">
        <div className="stat-box">
          <div className="num">{template.year}년</div>
          <div className="label">연도</div>
        </div>
        <div className="stat-box">
          <div className="num">{template.fields?.length || 0}</div>
          <div className="label">메일머지 필드</div>
        </div>
        <div className="stat-box">
          <div className="num">{runs.length}</div>
          <div className="label">실행 횟수</div>
        </div>
      </div>

      {/* 필드 목록 */}
      <div className="doc p-5 mb-5">
        <div className="section-head mb-3">
          <h2>메일머지 필드</h2>
          <span className="meta">{template.fields?.length || 0}개</span>
        </div>
        {template.fields && template.fields.length > 0 ? (
          <table className="doc-table">
            <thead>
              <tr>
                <th>필드명</th>
                <th>표시명</th>
                <th>유형</th>
                <th>옵션</th>
              </tr>
            </thead>
            <tbody>
              {template.fields.map((f: any, i: number) => (
                <tr key={i}>
                  <td className="font-mono text-sm font-bold">{f.name}</td>
                  <td>{f.label}</td>
                  <td className="text-xs" style={{ color: 'var(--muted)' }}>{f.type}</td>
                  <td className="text-xs" style={{ color: 'var(--muted)' }}>
                    {f.options ? (Array.isArray(f.options) ? f.options.join(', ') : f.options) : '—'}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <div className="doc-empty">등록된 필드가 없습니다.</div>
        )}
      </div>

      {/* 실행 기록 */}
      <div className="doc p-5">
        <div className="section-head mb-3">
          <h2>실행 기록</h2>
          <span className="meta">총 {runs.length}회</span>
        </div>
        {runs.length === 0 ? (
          <div className="doc-empty">아직 실행 기록이 없습니다.</div>
        ) : (
          <table className="doc-table">
            <thead>
              <tr>
                <th>실행명</th>
                <th>대상</th>
                <th>인원</th>
                <th>일시</th>
                <th>상태</th>
              </tr>
            </thead>
            <tbody>
              {runs.map(run => (
                <tr key={run.id}>
                  <td><Link href={`/mailmerge/runs/${run.id}`} className="doc-link">{run.title}</Link></td>
                  <td className="text-xs" style={{ color: 'var(--muted)' }}>
                    {run.grade ? `${run.grade}학년 ` : ''}{run.classNum ? `${run.classNum}반` : ''}
                  </td>
                  <td className="text-xs">{run.studentCount}명</td>
                  <td className="text-xs" style={{ color: 'var(--muted)' }}>
                    {new Date(run.createdAt).toLocaleDateString('ko-KR')}
                  </td>
                  <td>
                    <span className={`doc-tag ${
                      run.status === 'completed' ? 'green' :
                      run.status === 'processing' ? 'blue' :
                      run.status === 'failed' ? 'red' :
                      run.status === 'data_ready' ? 'amber' : 'gray'
                    }`}>{mailmergeStatusLabel(run.status)}</span>
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
