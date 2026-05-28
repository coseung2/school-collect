'use client'

import { useState, useEffect } from 'react'
import { useParams, useRouter } from 'next/navigation'
import type { CollectRun, FormField } from '@/lib/types'

interface RunDetail extends Omit<CollectRun, 'template'> {
  template: {
    id: string
    title: string
    description: string | null
    year: number
    created_at: string
    updated_at: string
    fields: FormField[]
  }
}

export default function CollectFormPage() {
  const { id } = useParams<{ id: string }>()
  const router = useRouter()

  const [run, setRun] = useState<RunDetail | null>(null)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [grade, setGrade] = useState('')
  const [classNum, setClassNum] = useState('')
  const [submitter, setSubmitter] = useState('')
  const [answers, setAnswers] = useState<Record<string, string>>({})
  const [submitted, setSubmitted] = useState(false)

  useEffect(() => {
    Promise.all([
      fetch(`/api/runs?status=open`).then(r => r.json()),
      fetch(`/api/templates`).then(r => r.json()),
    ]).then(([runs, templates]) => {
      const runData = runs.find((r: any) => r.id === id)
      if (runData) {
        const template = templates.find((t: any) => t.id === runData.templateId || t.id === runData.template?.id)
        setRun({ ...runData, template: template || runData.template })
      }
      setLoading(false)
    })
  }, [id])

  // 마감 확인
  useEffect(() => {
    if (run && new Date(run.deadline) < new Date()) {
      setSubmitted(true)
    }
  }, [run])

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    if (!run) return
    setSaving(true)

    try {
      const res = await fetch('/api/submissions', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          runId: id,
          grade: grade ? parseInt(grade) : null,
          classNum: classNum ? parseInt(classNum) : null,
          submitter,
          answers: Object.entries(answers).map(([fieldId, value]) => ({ fieldId, value })),
        })
      })

      if (!res.ok) {
        const err = await res.json()
        throw new Error(err.error)
      }

      setSubmitted(true)
    } catch (err: any) {
      alert('제출 실패: ' + err.message)
    } finally {
      setSaving(false)
    }
  }

  if (loading) return <div className="card text-center py-8 text-warm-400">로딩중...</div>
  if (!run) return <div className="card text-center py-8 text-warm-400">수합을 찾을 수 없습니다</div>

  if (submitted) {
    return (
      <div className="text-center py-12">
        <div className="text-4xl mb-3">✅</div>
        <h2 className="text-lg font-bold text-warm-700 mb-2">
          {run.template?.title || '수합'} — 제출 완료
        </h2>
        <p className="text-sm text-warm-500 mb-4">입력해주셔서 감사합니다</p>
        <button onClick={() => router.push('/')} className="btn btn-primary">
          대시보드로
        </button>
      </div>
    )
  }

  const isExpired = new Date(run.deadline) < new Date()

  return (
    <div className="max-w-2xl mx-auto">
      <div className="mb-4">
        <h1 className="text-xl font-bold text-warm-800">{run.template?.title || '수합'}</h1>
        {run.template?.description && (
          <p className="text-sm text-warm-500 mt-1">{run.template.description}</p>
        )}
        <div className="flex items-center gap-4 mt-2 text-xs text-warm-500">
          <span>📅 마감: {new Date(run.deadline).toLocaleDateString('ko-KR')}</span>
          {isExpired && <span className="text-red-500 font-medium">⚠️ 마감 지남</span>}
        </div>
      </div>

      <form onSubmit={handleSubmit} className="card space-y-4">
        {/* 제출자 정보 */}
        <div className="grid grid-cols-3 gap-3 pb-4 border-b border-warm-100">
          <div>
            <label className="label label-required">학년</label>
            <input className="input" type="number" min={1} max={6}
              value={grade} onChange={e => setGrade(e.target.value)} required />
          </div>
          <div>
            <label className="label label-required">반</label>
            <input className="input" type="number" min={1} max={20}
              value={classNum} onChange={e => setClassNum(e.target.value)} required />
          </div>
          <div>
            <label className="label">작성자</label>
            <input className="input" value={submitter}
              onChange={e => setSubmitter(e.target.value)} placeholder="이름" />
          </div>
        </div>

        {/* 필드별 입력 */}
        {(run.template?.fields || [])
          .sort((a, b) => a.order - b.order)
          .map(field => (
            <div key={field.id}>
              <label className={`label ${field.required ? 'label-required' : ''}`}>
                {field.label}
              </label>

              {field.fieldType === 'text' && (
                <input className="input" value={answers[field.id] || ''}
                  onChange={e => setAnswers({ ...answers, [field.id]: e.target.value })}
                  required={field.required} />
              )}
              {field.fieldType === 'number' && (
                <input className="input" type="number" value={answers[field.id] || ''}
                  onChange={e => setAnswers({ ...answers, [field.id]: e.target.value })}
                  required={field.required} />
              )}
              {field.fieldType === 'date' && (
                <input className="input" type="date" value={answers[field.id] || ''}
                  onChange={e => setAnswers({ ...answers, [field.id]: e.target.value })}
                  required={field.required} />
              )}
              {field.fieldType === 'textarea' && (
                <textarea className="input" rows={3} value={answers[field.id] || ''}
                  onChange={e => setAnswers({ ...answers, [field.id]: e.target.value })}
                  required={field.required} />
              )}
              {field.fieldType === 'select' && field.options && (
                <select className="select" value={answers[field.id] || ''}
                  onChange={e => setAnswers({ ...answers, [field.id]: e.target.value })}
                  required={field.required}>
                  <option value="">— 선택 —</option>
                  {JSON.parse(field.options).map((opt: string) => (
                    <option key={opt} value={opt}>{opt}</option>
                  ))}
                </select>
              )}
            </div>
          ))}

        <div className="pt-3 border-t border-warm-100 flex gap-3">
          <button type="submit" disabled={saving || isExpired} className="btn btn-primary">
            {saving ? '저장중...' : isExpired ? '마감됨' : '제출하기'}
          </button>
          <button type="button" onClick={() => router.push('/')} className="btn btn-secondary">
            취소
          </button>
        </div>
      </form>
    </div>
  )
}
