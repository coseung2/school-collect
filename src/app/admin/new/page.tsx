'use client'

import { useState, useEffect } from 'react'
import { useRouter } from 'next/navigation'
import { fieldTypeLabel } from '@/lib/types'
import type { FormTemplate, FormField } from '@/lib/types'

export default function NewCollectPage() {
  const router = useRouter()
  const [step, setStep] = useState<'template' | 'run'>('template')
  const [templates, setTemplates] = useState<FormTemplate[]>([])
  const [selectedTemplate, setSelectedTemplate] = useState<string>('')
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)

  // 새 템플릿 폼
  const [newTitle, setNewTitle] = useState('')
  const [newDesc, setNewDesc] = useState('')
  const [fields, setFields] = useState<Partial<FormField>[]>([
    { label: '', fieldType: 'text', required: false, order: 0 }
  ])

  // 실행 설정
  const [deadline, setDeadline] = useState('')
  const [targetType, setTargetType] = useState('all')
  const [targetValue, setTargetValue] = useState('')

  useEffect(() => {
    fetch('/api/templates').then(r => r.json()).then(data => {
      setTemplates(data)
      setLoading(false)
    })
  }, [])

  function addField() {
    setFields([...fields, { label: '', fieldType: 'text', required: false, order: fields.length }])
  }

  function removeField(i: number) {
    setFields(fields.filter((_, idx) => idx !== i))
  }

  function updateField(i: number, key: string, value: any) {
    const updated = [...fields]
    updated[i] = { ...updated[i], [key]: value }
    setFields(updated)
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    setSaving(true)

    try {
      // 템플릿 생성 또는 선택
      let templateId = selectedTemplate

      if (step === 'template' && !selectedTemplate) {
        const validFields = fields.filter(f => f.label?.trim())
        const res = await fetch('/api/templates', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            title: newTitle,
            description: newDesc,
            year: new Date().getFullYear(),
            fields: validFields.map((f, i) => ({ ...f, order: i })),
          })
        })
        const template = await res.json()
        templateId = template.id
      }

      // 수합 실행 생성
      const targetVal = targetType === 'all' ? null
        : targetType === 'grade' ? JSON.stringify(targetValue.split(',').map(s => s.trim()))
        : targetValue

      const runRes = await fetch('/api/runs', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          templateId,
          year: new Date().getFullYear(),
          deadline: new Date(deadline).toISOString(),
          targetType,
          targetValue: targetVal ? JSON.stringify(targetVal) : null,
          description: newDesc || null,
        })
      })
      const run = await runRes.json()
      router.push(`/collect/${run.id}`)
    } catch (err) {
      alert('저장 실패: ' + err)
    } finally {
      setSaving(false)
    }
  }

  if (loading) return <div className="card text-center py-8 text-warm-400">로딩중...</div>

  return (
    <div>
      <h1 className="text-xl font-bold text-warm-800 mb-6">새 수합 등록</h1>

      <form onSubmit={handleSubmit} className="max-w-2xl space-y-6">

        {/* 템플릿 선택 or 신규 */}
        <div className="card space-y-4">
          <h2 className="font-semibold text-warm-700 text-sm">1. 양식 선택</h2>

          {templates.length > 0 && (
            <div>
              <label className="label">기존 양식 재사용</label>
              <select className="select" value={selectedTemplate}
                onChange={e => setSelectedTemplate(e.target.value)}>
                <option value="">— 새로 만들기 —</option>
                {templates.map(t => (
                  <option key={t.id} value={t.id}>
                    {t.title} ({t.year})
                  </option>
                ))}
              </select>
            </div>
          )}

          {!selectedTemplate && (
            <>
              <div>
                <label className="label label-required">수합명</label>
                <input className="input" value={newTitle} onChange={e => setNewTitle(e.target.value)}
                  placeholder="예: 학급별 도서구입목록" required />
              </div>
              <div>
                <label className="label">설명</label>
                <textarea className="input" value={newDesc} onChange={e => setNewDesc(e.target.value)}
                  placeholder="어떤 내용인지 간단히" rows={2} />
              </div>

              {/* 필드 목록 */}
              <div>
                <label className="label label-required">입력 항목</label>
                <div className="space-y-2">
                  {fields.map((f, i) => (
                    <div key={i} className="flex items-center gap-2">
                      <input className="input flex-1" value={f.label}
                        onChange={e => updateField(i, 'label', e.target.value)}
                        placeholder={`항목 ${i + 1}`} />
                      <select className="select w-28" value={f.fieldType}
                        onChange={e => updateField(i, 'fieldType', e.target.value)}>
                        {['text', 'number', 'date', 'select', 'textarea'].map(t => (
                          <option key={t} value={t}>{fieldTypeLabel(t)}</option>
                        ))}
                      </select>
                      <label className="flex items-center gap-1 text-xs text-warm-500 whitespace-nowrap">
                        <input type="checkbox" checked={f.required || false}
                          onChange={e => updateField(i, 'required', e.target.checked)} />
                        필수
                      </label>
                      {fields.length > 1 && (
                        <button type="button" onClick={() => removeField(i)}
                          className="text-red-400 hover:text-red-600 text-sm px-1">×</button>
                      )}
                    </div>
                  ))}
                </div>
                <button type="button" onClick={addField}
                  className="btn btn-ghost text-xs mt-2 px-3 py-1">+ 항목 추가</button>
              </div>
            </>
          )}
        </div>

        {/* 실행 설정 */}
        <div className="card space-y-4">
          <h2 className="font-semibold text-warm-700 text-sm">2. 실행 설정</h2>

          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="label label-required">마감일</label>
              <input className="input" type="date" value={deadline}
                onChange={e => setDeadline(e.target.value)} required />
            </div>
            <div>
              <label className="label label-required">대상</label>
              <select className="select" value={targetType} onChange={e => setTargetType(e.target.value)}>
                <option value="all">전체 학년·반</option>
                <option value="grade">특정 학년</option>
                <option value="class">특정 반</option>
              </select>
            </div>
          </div>

          {targetType === 'grade' && (
            <div>
              <label className="label">학년 (쉼표로 구분)</label>
              <input className="input" value={targetValue}
                onChange={e => setTargetValue(e.target.value)} placeholder="예: 1,2,3" />
            </div>
          )}
          {targetType === 'class' && (
            <div>
              <label className="label">반 (쉼표로 구분)</label>
              <input className="input" value={targetValue}
                onChange={e => setTargetValue(e.target.value)} placeholder="예: 1-1,1-2,2-1" />
            </div>
          )}
        </div>

        <div className="flex gap-3">
          <button type="submit" disabled={saving} className="btn btn-primary">
            {saving ? '저장중...' : '수합 등록하기'}
          </button>
          <button type="button" onClick={() => router.back()} className="btn btn-secondary">취소</button>
        </div>
      </form>
    </div>
  )
}
