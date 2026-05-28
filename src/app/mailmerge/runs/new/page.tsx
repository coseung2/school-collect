'use client'

import { Suspense, useState, useEffect } from 'react'
import { useRouter, useSearchParams } from 'next/navigation'
import Link from 'next/link'
import type { MailmergeTemplate } from '@/lib/types'

function NewRunForm() {
  const router = useRouter()
  const searchParams = useSearchParams()
  const preselectedTemplate = searchParams.get('template')

  const [templates, setTemplates] = useState<MailmergeTemplate[]>([])
  const [templateId, setTemplateId] = useState(preselectedTemplate || '')
  const [title, setTitle] = useState('')
  const [grade, setGrade] = useState('')
  const [classNum, setClassNum] = useState('')
  const [studentCount, setStudentCount] = useState('')
  const [dataFilePath, setDataFilePath] = useState('')
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    fetch('/api/mailmerge/templates?year=' + new Date().getFullYear())
      .then(r => r.json())
      .then(data => {
        setTemplates(data || [])
        setLoading(false)
      })
      .catch(() => setLoading(false))
  }, [])

  const selectedTemplate = templates.find(t => t.id === templateId)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setSaving(true)
    setError('')

    if (!templateId) { setError('템플릿을 선택하세요.'); setSaving(false); return }
    if (!title.trim()) { setError('실행 제목을 입력하세요.'); setSaving(false); return }

    const res = await fetch('/api/mailmerge/runs', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        templateId,
        title: title.trim(),
        grade: grade ? parseInt(grade) : null,
        classNum: classNum ? parseInt(classNum) : null,
        studentCount: studentCount ? parseInt(studentCount) : 0,
        dataFilePath: dataFilePath.trim() || null,
      }),
    })

    if (!res.ok) {
      const err = await res.json()
      setError(err.error || '생성 실패')
      setSaving(false)
      return
    }

    const data = await res.json()
    router.push(`/mailmerge/runs/${data.id}`)
  }

  return (
    <div className="max-w-2xl">
      <Link href="/mailmerge" className="doc-link text-xs mb-4 inline-block">← 메일머지</Link>

      <div className="doc p-5">
        <div className="doc-header">
          <div className="sub">🚀 새 작업</div>
          <h1>메일머지 실행</h1>
        </div>

        <form onSubmit={handleSubmit} className="space-y-5">
          {error && (
            <div className="doc-tag red block text-xs" style={{ padding: '8px 10px' }}>{error}</div>
          )}

          {/* 1. 템플릿 선택 */}
          <div>
            <div className="section-head">
              <h2>1. 템플릿 선택</h2>
            </div>
            {loading ? (
              <div className="text-xs" style={{ color: 'var(--muted)' }}>로딩 중...</div>
            ) : templates.length === 0 ? (
              <div className="text-xs" style={{ color: 'var(--muted)' }}>
                등록된 템플릿이 없습니다.{' '}
                <Link href="/mailmerge/templates/new" className="doc-link">템플릿 등록하기</Link>
              </div>
            ) : (
              <table className="doc-table">
                <tbody>
                  <tr>
                    <td className="doc-label-cell">템플릿</td>
                    <td>
                      <select value={templateId} onChange={e => {
                        setTemplateId(e.target.value)
                        const tpl = templates.find(t => t.id === e.target.value)
                        if (tpl && !title) setTitle(tpl.title)
                      }}
                        className="doc-input w-full text-sm">
                        <option value="">— 선택 —</option>
                        {templates.map(t => (
                          <option key={t.id} value={t.id}>
                            {t.title} ({t.year}년)
                          </option>
                        ))}
                      </select>
                    </td>
                  </tr>
                </tbody>
              </table>
            )}
            {selectedTemplate && selectedTemplate.fields && selectedTemplate.fields.length > 0 && (
              <div className="flex flex-wrap gap-1 mt-2">
                {selectedTemplate.fields.map((f: any, i: number) => (
                  <span key={i} className="doc-tag gray">{f.name}</span>
                ))}
              </div>
            )}
          </div>

          {/* 2. 실행 정보 */}
          <div>
            <div className="section-head">
              <h2>2. 실행 정보</h2>
            </div>
            <table className="doc-table">
              <tbody>
                <tr>
                  <td className="doc-label-cell">실행 제목</td>
                  <td>
                    <input type="text" value={title} onChange={e => setTitle(e.target.value)}
                      className="doc-input w-full text-sm p-0"
                      placeholder="예: 5학년 3반 신체검사 가정통신문" />
                  </td>
                </tr>
                <tr>
                  <td className="doc-label-cell">대상</td>
                  <td>
                    <div className="flex gap-2 items-center">
                      <input type="number" value={grade} onChange={e => setGrade(e.target.value)}
                        className="doc-input text-sm w-16"
                        placeholder="5" />
                      <span className="text-xs" style={{ color: 'var(--muted)' }}>학년</span>
                      <input type="number" value={classNum} onChange={e => setClassNum(e.target.value)}
                        className="doc-input text-sm w-16"
                        placeholder="3" />
                      <span className="text-xs" style={{ color: 'var(--muted)' }}>반</span>
                      <span className="text-xs" style={{ color: 'var(--muted)' }}>·</span>
                      <input type="number" value={studentCount} onChange={e => setStudentCount(e.target.value)}
                        className="doc-input text-sm w-16"
                        placeholder="24" />
                      <span className="text-xs" style={{ color: 'var(--muted)' }}>명</span>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          {/* 3. 데이터 파일 */}
          <div>
            <div className="section-head">
              <h2>3. 데이터 파일</h2>
            </div>
            <table className="doc-table">
              <tbody>
                <tr>
                  <td className="doc-label-cell">CSV 경로</td>
                  <td>
                    <input type="text" value={dataFilePath} onChange={e => setDataFilePath(e.target.value)}
                      className="doc-input w-full p-0 font-mono text-xs"
                      placeholder="인박스/ocr_결과.csv" />
                  </td>
                </tr>
              </tbody>
            </table>
            <div className="text-xs mt-1" style={{ color: 'var(--muted)' }}>
              실제 CSV 처리는 에이전트가 수행합니다. 여기서는 참조 경로만 기록합니다.
            </div>
          </div>

          {/* 버튼 */}
          <div className="toolbar" style={{ borderBottom: 'none', marginBottom: 0, padding: 0 }}>
            <button type="submit" disabled={saving || !templateId} className="doc-btn primary">
              {saving ? '생성 중...' : '실행 등록'}
            </button>
            <Link href="/mailmerge" className="doc-btn">취소</Link>
          </div>
        </form>
      </div>
    </div>
  )
}

export default function NewRunPage() {
  return (
    <Suspense fallback={<div className="doc-empty">로딩 중...</div>}>
      <NewRunForm />
    </Suspense>
  )
}
