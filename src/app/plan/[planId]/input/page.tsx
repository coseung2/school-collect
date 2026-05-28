'use client'

import { useState, useEffect, useCallback } from 'react'
import { useParams, useRouter } from 'next/navigation'
import { dayOfWeekLabel } from '@/lib/types'
import type { MonthlyPlan, MonthlyPlanEntry } from '@/lib/types'

interface EntryRow {
  id?: string
  planDate: string
  content: string
  target: string
  location: string
  personInCharge: string
  notes: string
  status: string
}

export default function PlanInputPage() {
  const { planId } = useParams<{ planId: string }>()
  const router = useRouter()
  const [plan, setPlan] = useState<MonthlyPlan | null>(null)
  const [entries, setEntries] = useState<EntryRow[]>([])
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [submitter, setSubmitter] = useState('')
  const [submitted, setSubmitted] = useState(false)

  // Load existing entries
  useEffect(() => {
    Promise.all([
      fetch(`/api/plans?id=${planId}`).then(r => r.json()),
      fetch(`/api/entries?planId=${planId}`).then(r => r.json()),
    ]).then(([planData, entryData]) => {
      setPlan(planData)
      const existing = (entryData || []) as MonthlyPlanEntry[]
      if (existing.length > 0) {
        setEntries(existing.map(e => ({
          id: e.id,
          planDate: e.planDate.slice(0, 10),
          content: e.content,
          target: e.target,
          location: e.location,
          personInCharge: e.personInCharge,
          notes: e.notes,
          status: e.status,
        })))
        if (existing[0].submitter) setSubmitter(existing[0].submitter)
        const allSubmitted = existing.every(e => e.status === 'submitted' || e.status === 'confirmed' || e.status === 'rejected')
        if (allSubmitted && existing.length > 0) setSubmitted(true)
      } else {
        // Initialize with next Monday by default
        const nextMon = new Date()
        nextMon.setDate(nextMon.getDate() + ((8 - nextMon.getDay()) % 7 || 7))
        setEntries([{ planDate: nextMon.toISOString().slice(0, 10), content: '', target: '', location: '', personInCharge: '', notes: '', status: 'draft' }])
      }
      setLoading(false)
    })
  }, [planId])

  const addRow = useCallback(() => {
    setEntries(prev => [...prev, { planDate: '', content: '', target: '', location: '', personInCharge: '', notes: '', status: 'draft' }])
  }, [])

  const removeRow = useCallback((i: number) => {
    setEntries(prev => prev.length > 1 ? prev.filter((_, idx) => idx !== i) : prev.map(e => ({ ...e, planDate: '', content: '', target: '', location: '', personInCharge: '', notes: '' })))
  }, [])

  const updateRow = useCallback((i: number, key: keyof EntryRow, value: string) => {
    setEntries(prev => prev.map((e, idx) => idx === i ? { ...e, [key]: value } : e))
  }, [])

  const isExpired = plan?.deadline ? new Date(plan.deadline) < new Date() : false

  async function handleSave(draft: boolean) {
    if (!plan) return
    setSaving(true)

    try {
      // Delete removed entries
      const currentIds = entries.filter(e => e.id).map(e => e.id!)
      const existingEntries = await (await fetch(`/api/entries?planId=${planId}`)).json()
      for (const existing of existingEntries) {
        if (!currentIds.includes(existing.id)) {
          await fetch(`/api/entries?id=${existing.id}`, { method: 'DELETE' })
        }
      }

      // Upsert all rows as batch
      const payload = {
        planId,
        submitter,
        entries: entries.map(e => ({
          ...(e.id ? { id: e.id } : {}),
          planDate: e.planDate,
          content: e.content,
          target: e.target,
          location: e.location,
          personInCharge: e.personInCharge,
          notes: e.notes,
          status: draft ? 'draft' : 'submitted',
        })),
      }

      // Send non-empty rows
      const validRows = payload.entries.filter(e => e.content && e.planDate)
      const res = await fetch('/api/entries', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ ...payload, entries: validRows }),
      })

      if (!res.ok) throw new Error(await res.text())
      if (!draft) setSubmitted(true)
      alert(draft ? '임시저장 완료' : '제출 완료')
    } catch (err: any) {
      alert('저장 실패: ' + err.message)
    } finally {
      setSaving(false)
    }
  }

  // Group by week
  const weekGroups = entries.reduce<{ label: string; rows: EntryRow[] }[]>((acc, e) => {
    if (!e.planDate) {
      if (acc.length === 0 || acc[acc.length - 1].label !== '(날짜 없음)') {
        acc.push({ label: '(날짜 없음)', rows: [] })
      }
      acc[acc.length - 1].rows.push(e)
      return acc
    }
    const d = new Date(e.planDate)
    const yearStart = new Date(d.getFullYear(), 0, 1)
    const weekNum = Math.ceil(((d.getTime() - yearStart.getTime()) / 86400000 + yearStart.getDay() + 1) / 7)
    const label = `${d.getMonth() + 1}월 ${weekNum}주차`
    if (acc.length === 0 || acc[acc.length - 1].label !== label) {
      acc.push({ label, rows: [] })
    }
    acc[acc.length - 1].rows.push(e)
    return acc
  }, [])

  if (loading) return <div className="p-6 text-gray-400 text-sm">로딩중...</div>
  if (!plan) return <div className="p-6 text-gray-400 text-sm">월중계획을 찾을 수 없습니다</div>

  if (submitted) {
    return (
      <div className="max-w-3xl mx-auto text-center py-12">
        <div className="text-3xl mb-3">✅</div>
        <h2 className="text-lg font-bold text-gray-700 mb-2">제출 완료</h2>
        <p className="text-sm text-gray-500 mb-4">월중계획 입력이 완료되었습니다</p>
        <button onClick={() => router.push('/')} className="px-5 py-2 text-sm font-medium text-white bg-gray-800 hover:bg-gray-900" style={{ borderRadius: 0 }}>
          대시보드로
        </button>
      </div>
    )
  }

  return (
    <div style={{ maxWidth: 1000, margin: '0 auto' }}>
      <div className="mb-5 pb-4 border-b border-gray-200">
        <h1 className="text-lg font-bold text-gray-800 mb-1">{plan.title || `${plan.year}년 ${plan.month}월 월중계획`} — 입력</h1>
        {plan.description && <p className="text-sm text-gray-500">{plan.description}</p>}
        {plan.deadline && (
          <p className={`text-xs mt-1 ${isExpired ? 'text-red-500' : 'text-gray-500'}`}>
            📅 마감: {new Date(plan.deadline).toLocaleDateString('ko-KR')}
            {isExpired && ' (마감 지남)'}
          </p>
        )}
      </div>

      <div className="mb-4">
        <label className="block text-xs font-medium text-gray-600 mb-1">작성자</label>
        <input className="border border-gray-200 px-3 py-2 text-sm bg-white w-48" style={{ borderRadius: 0 }}
          value={submitter} onChange={e => setSubmitter(e.target.value)}
          placeholder="이름을 입력하세요" />
      </div>

      {/* Week Grouped Table */}
      {weekGroups.map(group => (
        <div key={group.label} className="mb-6">
          <div className="bg-gray-50 border border-gray-200 border-b-0 px-4 py-2">
            <span className="text-xs font-semibold text-gray-600">{group.label}</span>
          </div>
          <div className="border border-gray-200" style={{ borderRadius: 0 }}>
            <table className="w-full text-xs border-collapse">
              <thead>
                <tr className="bg-gray-50 border-b border-gray-200">
                  <th className="text-left px-3 py-2 font-medium text-gray-500 w-[100px]">날짜</th>
                  <th className="text-left px-3 py-2 font-medium text-gray-500 w-[140px]">요일</th>
                  <th className="text-left px-3 py-2 font-medium text-gray-500">추진 내용</th>
                  <th className="text-left px-3 py-2 font-medium text-gray-500 w-[100px]">대상</th>
                  <th className="text-left px-3 py-2 font-medium text-gray-500 w-[100px]">장소</th>
                  <th className="text-left px-3 py-2 font-medium text-gray-500 w-[90px]">담당</th>
                  <th className="text-left px-3 py-2 font-medium text-gray-500 w-[120px]">비고</th>
                  <th className="w-[30px]"></th>
                </tr>
              </thead>
              <tbody>
                {group.rows.map((row, ri) => {
                  const globalIdx = entries.indexOf(row)
                  const dow = row.planDate ? new Date(row.planDate + 'T12:00:00').getDay() : -1
                  return (
                    <tr key={globalIdx} className="border-b border-gray-100 hover:bg-gray-50">
                      <td className="px-3 py-1.5">
                        <input type="date" className="w-full border border-gray-200 px-2 py-1 text-xs bg-white"
                          style={{ borderRadius: 0 }}
                          value={row.planDate}
                          onChange={e => updateRow(globalIdx, 'planDate', e.target.value)} />
                      </td>
                      <td className="px-3 py-1.5 text-gray-500">
                        {dow >= 0 ? dayOfWeekLabel(dow, true) : ''}
                      </td>
                      <td className="px-3 py-1.5">
                        <input className="w-full border border-gray-200 px-2 py-1 text-xs bg-white"
                          style={{ borderRadius: 0 }}
                          value={row.content}
                          onChange={e => updateRow(globalIdx, 'content', e.target.value)}
                          placeholder="추진 내용" />
                      </td>
                      <td className="px-3 py-1.5">
                        <input className="w-full border border-gray-200 px-2 py-1 text-xs bg-white"
                          style={{ borderRadius: 0 }}
                          value={row.target}
                          onChange={e => updateRow(globalIdx, 'target', e.target.value)}
                          placeholder="대상" />
                      </td>
                      <td className="px-3 py-1.5">
                        <input className="w-full border border-gray-200 px-2 py-1 text-xs bg-white"
                          style={{ borderRadius: 0 }}
                          value={row.location}
                          onChange={e => updateRow(globalIdx, 'location', e.target.value)}
                          placeholder="장소" />
                      </td>
                      <td className="px-3 py-1.5">
                        <input className="w-full border border-gray-200 px-2 py-1 text-xs bg-white"
                          style={{ borderRadius: 0 }}
                          value={row.personInCharge}
                          onChange={e => updateRow(globalIdx, 'personInCharge', e.target.value)}
                          placeholder="담당" />
                      </td>
                      <td className="px-3 py-1.5">
                        <input className="w-full border border-gray-200 px-2 py-1 text-xs bg-white"
                          style={{ borderRadius: 0 }}
                          value={row.notes}
                          onChange={e => updateRow(globalIdx, 'notes', e.target.value)}
                          placeholder="비고" />
                      </td>
                      <td className="px-3 py-1.5">
                        <button onClick={() => removeRow(globalIdx)}
                          className="text-red-400 hover:text-red-600 text-xs px-1"
                          title="삭제">×</button>
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
            <div className="px-3 py-2 border-t border-gray-200 bg-gray-50">
              <button onClick={addRow}
                className="px-3 py-1 text-xs font-medium text-gray-600 bg-white border border-gray-200 hover:bg-gray-50"
                style={{ borderRadius: 0 }}>
                + 행 추가
              </button>
            </div>
          </div>
        </div>
      ))}

      {/* Action buttons */}
      <div className="flex gap-3 mt-6 pb-8">
        <button onClick={() => handleSave(false)} disabled={saving || isExpired || !submitter.trim()}
          className="px-6 py-2.5 text-sm font-medium text-white bg-gray-800 hover:bg-gray-900 disabled:opacity-50"
          style={{ borderRadius: 0 }}>
          {saving ? '저장중...' : isExpired ? '마감됨' : '제출하기'}
        </button>
        <button onClick={() => handleSave(true)} disabled={saving}
          className="px-6 py-2.5 text-sm font-medium text-gray-600 bg-gray-100 border border-gray-200 hover:bg-gray-200"
          style={{ borderRadius: 0 }}>
          임시저장
        </button>
        <button onClick={() => router.back()}
          className="px-6 py-2.5 text-sm font-medium text-gray-500 hover:bg-gray-50 border border-gray-200"
          style={{ borderRadius: 0 }}>
          취소
        </button>
      </div>
    </div>
  )
}
