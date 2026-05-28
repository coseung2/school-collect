'use client'

import { useState } from 'react'
import { useRouter } from 'next/navigation'
import Link from 'next/link'
import { docTypeLabel } from '@/lib/types'

const DOC_TYPES = ['general', 'health_check', 'dental', 'urine', 'vision', 'grade_report', 'consent', 'notice', 'etc'] as const

interface FieldInput {
  name: string
  label: string
  type: 'text' | 'number' | 'select'
  options?: string
}

export default function NewTemplatePage() {
  const router = useRouter()
  const [title, setTitle] = useState('')
  const [description, setDescription] = useState('')
  const [docType, setDocType] = useState('general')
  const [year, setYear] = useState(new Date().getFullYear().toString())
  const [filePath, setFilePath] = useState('')
  const [fields, setFields] = useState<FieldInput[]>([
    { name: '이름', label: '이름', type: 'text' },
    { name: '학년', label: '학년', type: 'text' },
    { name: '반', label: '반', type: 'text' },
    { name: '번호', label: '번호', type: 'number' },
  ])
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState('')

  const addField = () => {
    setFields([...fields, { name: '', label: '', type: 'text' }])
  }

  const removeField = (i: number) => {
    setFields(fields.filter((_, idx) => idx !== i))
  }

  const updateField = (i: number, key: keyof FieldInput, val: string) => {
    const updated = [...fields]
    updated[i] = { ...updated[i], [key]: val }
    setFields(updated)
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setSaving(true)
    setError('')

    if (!title.trim()) { setError('템플릿 제목을 입력하세요.'); setSaving(false); return }

    const cleanFields = fields
      .filter(f => f.name.trim())
      .map(f => ({
        name: f.name.trim(),
        label: f.label.trim() || f.name.trim(),
        type: f.type,
        ...(f.type === 'select' && f.options?.trim() ? { options: f.options.split(',').map(s => s.trim()) } : {}),
      }))

    const res = await fetch('/api/mailmerge/templates', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        title: title.trim(),
        description: description.trim() || null,
        documentType: docType,
        year: parseInt(year) || new Date().getFullYear(),
        fields: cleanFields,
        templateFilePath: filePath.trim() || null,
      }),
    })

    if (!res.ok) {
      const err = await res.json()
      setError(err.error || '저장 실패')
      setSaving(false)
      return
    }

    const data = await res.json()
    router.push(`/mailmerge/templates/${data.id}`)
  }

  return (
    <div className="max-w-2xl">
      <Link href="/mailmerge" className="doc-link text-xs mb-4 inline-block">← 메일머지</Link>

      <div className="doc p-5">
        <div className="doc-header">
          <div className="sub">📄 새 템플릿</div>
          <h1>템플릿 등록</h1>
        </div>

        <form onSubmit={handleSubmit} className="space-y-5">
          {error && (
            <div className="doc-tag red block text-xs" style={{ padding: '8px 10px' }}>{error}</div>
          )}

          {/* 기본 정보 */}
          <div>
            <div className="section-head">
              <h2>기본 정보</h2>
            </div>
            <table className="doc-table">
              <tbody>
                <tr>
                  <td className="doc-label-cell">템플릿 제목</td>
                  <td>
                    <input type="text" value={title} onChange={e => setTitle(e.target.value)}
                      className="doc-input w-full text-sm p-0"
                      placeholder="예: 신체발달상황 및 시력검사 결과 안내" />
                  </td>
                </tr>
                <tr>
                  <td className="doc-label-cell">설명</td>
                  <td>
                    <input type="text" value={description} onChange={e => setDescription(e.target.value)}
                      className="doc-input w-full text-sm p-0"
                      placeholder="선택사항" />
                  </td>
                </tr>
                <tr>
                  <td className="doc-label-cell">문서 유형</td>
                  <td>
                    <select value={docType} onChange={e => setDocType(e.target.value)}
                      className="doc-input text-sm">
                      {DOC_TYPES.map(t => (
                        <option key={t} value={t}>{docTypeLabel(t)}</option>
                      ))}
                    </select>
                  </td>
                </tr>
                <tr>
                  <td className="doc-label-cell">연도</td>
                  <td>
                    <input type="number" value={year} onChange={e => setYear(e.target.value)}
                      className="doc-input text-sm w-20" />
                  </td>
                </tr>
                <tr>
                  <td className="doc-label-cell">파일 경로</td>
                  <td>
                    <input type="text" value={filePath} onChange={e => setFilePath(e.target.value)}
                      className="doc-input w-full p-0 font-mono text-xs"
                      placeholder="인박스/템플릿/..." />
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          {/* 필드 정의 */}
          <div>
            <div className="section-head">
              <h2>메일머지 필드</h2>
              <button type="button" onClick={addField} className="doc-btn ghost" style={{ fontSize: 12 }}>
                + 필드 추가
              </button>
            </div>
            <table className="doc-table">
              <thead>
                <tr>
                  <th className="w-2/5">필드명 ({'{{필드명}}'})</th>
                  <th className="w-2/5">표시명</th>
                  <th className="w-1/6">유형</th>
                  <th className="w-16"></th>
                </tr>
              </thead>
              <tbody>
                {fields.length === 0 ? (
                  <tr>
                    <td colSpan={4} className="doc-empty">필드가 없습니다. 추가 버튼을 눌러주세요.</td>
                  </tr>
                ) : fields.map((f, i) => (
                  <tr key={i}>
                    <td>
                      <input type="text" value={f.name} onChange={e => updateField(i, 'name', e.target.value)}
                        className="doc-input w-full text-sm p-0"
                        placeholder="필드명" />
                    </td>
                    <td>
                      <input type="text" value={f.label} onChange={e => updateField(i, 'label', e.target.value)}
                        className="doc-input w-full text-sm p-0"
                        placeholder="표시명" />
                    </td>
                    <td>
                      <select value={f.type} onChange={e => updateField(i, 'type', e.target.value)}
                        className="doc-input text-xs">
                        <option value="text">텍스트</option>
                        <option value="number">숫자</option>
                        <option value="select">선택</option>
                      </select>
                      {f.type === 'select' && (
                        <input type="text" value={f.options || ''} onChange={e => updateField(i, 'options', e.target.value)}
                          className="doc-input text-xs ml-1"
                          style={{ width: 60 }}
                          placeholder="예,아니오" />
                      )}
                    </td>
                    <td>
                      <button type="button" onClick={() => removeField(i)}
                        className="doc-tag red" style={{ cursor: 'pointer' }}>✕</button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            <div className="text-xs mt-2" style={{ color: 'var(--muted)' }}>
              템플릿 HWPX 파일의 {'{{필드명}}'} 과 일치해야 합니다.
            </div>
          </div>

          {/* 버튼 */}
          <div className="toolbar" style={{ borderBottom: 'none', marginBottom: 0, padding: 0 }}>
            <button type="submit" disabled={saving} className="doc-btn primary">
              {saving ? '저장 중...' : '템플릿 등록'}
            </button>
            <Link href="/mailmerge" className="doc-btn">취소</Link>
          </div>
        </form>
      </div>
    </div>
  )
}
