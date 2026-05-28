import Link from 'next/link'
import { getServiceSupabase } from '@/lib/supabase'
import type { MailmergeRun } from '@/lib/types'

async function getRun(id: string): Promise<MailmergeRun | null> {
  const sb = getServiceSupabase()
  const { data } = await sb.from('mailmerge_runs')
    .select('*, template:mailmerge_templates(*)')
    .eq('id', id)
    .single()
  if (!data) return null
  return {
    ...data,
    templateId: data.template_id,
    classNum: data.class_num,
    studentCount: data.student_count,
    dataFilePath: data.data_file_path,
    outputPath: data.output_path,
    errorMessage: data.error_message,
    createdAt: data.created_at,
    updatedAt: data.updated_at,
    template: data.template ? {
      ...data.template,
      documentType: data.template.document_type,
      templateFilePath: data.template.template_file_path,
      isActive: data.template.is_active,
      createdAt: data.template.created_at,
      updatedAt: data.template.updated_at,
    } : undefined,
  }
}

const statusStyle: Record<string, { tag: string; label: string; desc: string }> = {
  draft: { tag: 'gray', label: '준비중', desc: '⏳ PDF를 OCR하고 CSV 데이터를 준비하세요.' },
  data_ready: { tag: 'amber', label: '데이터 준비', desc: '✅ CSV가 준비되었습니다. 에이전트가 메일머지를 실행할 수 있습니다.' },
  processing: { tag: 'blue', label: '생성중', desc: '🔄 에이전트가 HWPX 파일을 생성 중입니다.' },
  completed: { tag: 'green', label: '완료', desc: '✅ 메일머지가 완료되었습니다. 출력물을 확인하세요.' },
  failed: { tag: 'red', label: '실패', desc: '⚠ 오류가 발생했습니다. 에러 메시지를 확인하세요.' },
}

export default async function RunDetailPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params
  const run = await getRun(id)

  if (!run) {
    return (
      <div className="doc-empty" style={{ paddingTop: 80 }}>
        <div className="text-base mb-2">실행 기록을 찾을 수 없습니다</div>
        <Link href="/mailmerge" className="doc-link">← 메일머지로 돌아가기</Link>
      </div>
    )
  }

  const st = statusStyle[run.status] || statusStyle.draft

  return (
    <div className="max-w-3xl">
      <Link href="/mailmerge" className="doc-link text-xs mb-4 inline-block">← 메일머지</Link>

      {/* 문서 헤더 */}
      <div className="doc p-5 mb-5">
        <div className="flex items-start justify-between">
          <div>
            <div className="sub mb-1">메일머지 실행</div>
            <h1 className="text-xl font-extrabold tracking-tight" style={{ color: 'var(--ink)' }}>{run.title}</h1>
            <div className="flex items-center gap-3 mt-2">
              <span className={`doc-tag ${st.tag}`}>{st.label}</span>
              <span className="text-xs" style={{ color: 'var(--muted)' }}>
                {new Date(run.createdAt).toLocaleDateString('ko-KR', {
                  year: 'numeric', month: 'long', day: 'numeric', hour: '2-digit', minute: '2-digit'
                })}
              </span>
            </div>
          </div>
        </div>
        <div className="mt-3 text-xs" style={{ color: 'var(--muted)', background: 'var(--head)', padding: '8px 10px', border: '1px solid var(--line)' }}>
          {st.desc}
        </div>
      </div>

      {run.errorMessage && (
        <div className="doc p-4 mb-5" style={{ borderColor: 'var(--red)', background: 'var(--red-weak)' }}>
          <div className="text-xs font-bold" style={{ color: 'var(--red)' }}>오류</div>
          <div className="text-xs mt-1" style={{ color: 'var(--red)' }}>{run.errorMessage}</div>
        </div>
      )}

      {/* 실행 정보 */}
      <div className="doc p-5 mb-5">
        <div className="section-head mb-3">
          <h2>실행 정보</h2>
        </div>
        <table className="doc-table">
          <tbody>
            <tr>
              <td className="doc-label-cell">템플릿</td>
              <td>
                {run.template ? (
                  <Link href={`/mailmerge/templates/${run.template.id}`} className="doc-link">
                    {run.template.title}
                  </Link>
                ) : '—'}
              </td>
            </tr>
            <tr>
              <td className="doc-label-cell">대상</td>
              <td>
                {run.grade ? `${run.grade}학년 ` : ''}
                {run.classNum ? `${run.classNum}반 ` : ''}
                ({run.studentCount}명)
              </td>
            </tr>
            {run.dataFilePath && (
              <tr>
                <td className="doc-label-cell">데이터 파일</td>
                <td className="font-mono text-xs">{run.dataFilePath}</td>
              </tr>
            )}
            {run.corrections && (
              <tr>
                <td className="doc-label-cell">수정 내역</td>
                <td className="text-xs">{run.corrections}</td>
              </tr>
            )}
            {run.outputPath && (
              <tr>
                <td className="doc-label-cell">출력 경로</td>
                <td className="font-mono text-xs">{run.outputPath}</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {/* 실행 가이드 */}
      <div className="doc p-5">
        <div className="section-head mb-3">
          <h2>실행 방법</h2>
        </div>
        <div style={{ fontSize: 13, color: 'var(--ink)' }}>
          {run.status === 'draft' && (
            <ol style={{ paddingLeft: 20, lineHeight: 2 }}>
              <li><strong>PDF OCR</strong> — 신체검사 PDF를 에이전트에 전달하여 데이터 추출</li>
              <li><strong>데이터 검증</strong> — 추출 결과 CSV를 확인하고 수정 요청</li>
              <li><strong>메일머지 생성</strong> — 템플릿 + CSV로 개인별 HWPX 생성</li>
            </ol>
          )}
          {run.status === 'completed' && (
            <div style={{ color: 'var(--green)' }}>
              ✅ 메일머지가 완료되었습니다. 출력물은 <strong>{run.outputPath || '출력 폴더'}</strong>에서 확인하세요.
            </div>
          )}
          {run.status === 'failed' && (
            <div style={{ color: 'var(--red)' }}>
              ⚠ 오류가 발생했습니다. 위 오류 메시지를 참고하여 다시 시도하세요.
            </div>
          )}
          <div className="mt-4 pt-3 text-xs" style={{ borderTop: '1px solid var(--line)', color: 'var(--muted)' }}>
            실제 HWPX 생성은 Hermes 에이전트가 수행합니다. 웹앱에서는 기록과 상태를 관리합니다.
          </div>
        </div>
      </div>
    </div>
  )
}
