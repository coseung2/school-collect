'use client'

import { useState, useEffect } from 'react'
import { useParams } from 'next/navigation'
import { entryStatusLabel, dayOfWeekLabel, planStatusLabel } from '@/lib/types'
import type { MonthlyPlan, MonthlyPlanEntry } from '@/lib/types'

export default function PlanReviewPage() {
  const { planId } = useParams<{ planId: string }>()
  const [plan, setPlan] = useState<MonthlyPlan | null>(null)
  const [allEntries, setAllEntries] = useState<MonthlyPlanEntry[]>([])
  const [loading, setLoading] = useState(true)
  const [statusFilter, setStatusFilter] = useState<string>('all')
  const [submitterFilter, setSubmitterFilter] = useState<string>('all')
  const [toast, setToast] = useState<string | null>(null)

  const showToast = (msg: string) => { setToast(msg); setTimeout(() => setToast(null), 2000) }

  useEffect(() => {
    Promise.all([
      fetch(`/api/plans?id=${planId}`).then(r => r.json()),
      fetch(`/api/entries?planId=${planId}`).then(r => r.json()),
    ]).then(([p, entries]) => {
      setPlan(p)
      setAllEntries(entries || [])
      setLoading(false)
    })
  }, [planId])

  async function updateEntryStatus(id: string, status: string) {
    const res = await fetch('/api/entries', {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ id, status }),
    })
    if (res.ok) {
      setAllEntries(prev => prev.map(e => e.id === id ? { ...e, status: status as any } : e))
      showToast(status === 'confirmed' ? '✅ 확정 완료' : '⛔ 반려됨')
    } else {
      showToast('❌ 상태 변경 실패')
    }
  }

  async function publishAll() {
    if (!confirm('모든 확정된 일정을 공개합니다. 계속할까요?')) return
    const confirmed = allEntries.filter(e => e.status === 'confirmed')
    for (const entry of confirmed) {
      await fetch('/api/entries', {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ id: entry.id, status: 'confirmed' }),
      })
    }
    await fetch('/api/plans', {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ id: planId, status: 'published' }),
    })
    setPlan(prev => prev ? { ...prev, status: 'published' } : prev)
    showToast('📢 확정 일정이 공개되었습니다')
  }

  // Stats
  const submitters = [...new Set(allEntries.map(e => e.submitter).filter(Boolean))]
  const statusCount = (s: string) => allEntries.filter(e => e.status === s).length
  const pendingCount = allEntries.filter(e => e.status === 'submitted').length
  const confirmedCount = statusCount('confirmed')
  const totalCount = allEntries.length

  // Detect conflicts (same date, same content)
  const conflicts = allEntries.filter((e, i, arr) =>
    e.status === 'submitted' &&
    arr.some((o, j) => j !== i && o.planDate === e.planDate && o.content === e.content)
  )

  const filtered = allEntries.filter(e => {
    if (statusFilter !== 'all' && e.status !== statusFilter) return false
    if (submitterFilter !== 'all' && e.submitter !== submitterFilter) return false
    return true
  })

  // Group by submitter then date
  const groupedBySubmitter = filtered.reduce<Record<string, MonthlyPlanEntry[]>>((acc, e) => {
    const key = e.submitter || '(미지정)'
    if (!acc[key]) acc[key] = []
    acc[key].push(e)
    return acc
  }, {})

  if (loading) return <div className="p-6 text-gray-400 text-sm">로딩중...</div>
  if (!plan) return <div className="p-6 text-gray-400 text-sm">월중계획을 찾을 수 없습니다</div>

  return (
    <div style={{ maxWidth: 1100, margin: '0 auto' }}>
      {/* Toast */}
      {toast && (
        <div className="fixed top-4 right-4 z-50 px-4 py-2 bg-black text-white text-xs" style={{ borderRadius: 0 }}>
          {toast}
        </div>
      )}

      {/* Header */}
      <div className="mb-5 pb-4 border-b border-gray-200">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-lg font-bold text-gray-800 mb-1">{plan.title || `${plan.year}년 ${plan.month}월 월중계획`} — 검토</h1>
            <span className="text-xs text-gray-500 bg-gray-100 px-2 py-0.5" style={{ borderRadius: 0 }}>
              {planStatusLabel(plan.status)}
            </span>
          </div>
          {plan.status !== 'published' && (
            <button onClick={publishAll}
              className="px-4 py-2 text-sm font-medium text-white bg-blue-700 hover:bg-blue-800"
              style={{ borderRadius: 0 }}>
              📢 확정 공개
            </button>
          )}
        </div>
      </div>

      {/* Status bar */}
      <div className="grid grid-cols-5 gap-3 mb-5">
        {[
          { label: '전체', count: totalCount, color: 'text-gray-800' },
          { label: '제출', count: statusCount('submitted'), color: 'text-yellow-700' },
          { label: '확정', count: confirmedCount, color: 'text-green-700' },
          { label: '반려', count: statusCount('rejected'), color: 'text-red-500' },
          { label: '검토대기', count: pendingCount, color: 'text-blue-700' },
        ].map(s => (
          <div key={s.label} className="border border-gray-200 bg-white px-4 py-3 text-center" style={{ borderRadius: 0 }}>
            <div className={`text-lg font-bold ${s.color}`}>{s.count}</div>
            <div className="text-[10px] text-gray-500 mt-0.5">{s.label}</div>
          </div>
        ))}
      </div>

      {/* Filters */}
      <div className="flex items-center gap-3 mb-4">
        <select className="border border-gray-200 px-3 py-2 text-xs bg-white" style={{ borderRadius: 0 }}
          value={statusFilter} onChange={e => setStatusFilter(e.target.value)}>
          <option value="all">전체 상태</option>
          <option value="draft">작성중</option>
          <option value="submitted">제출</option>
          <option value="confirmed">확정</option>
          <option value="rejected">반려</option>
        </select>
        <select className="border border-gray-200 px-3 py-2 text-xs bg-white" style={{ borderRadius: 0 }}
          value={submitterFilter} onChange={e => setSubmitterFilter(e.target.value)}>
          <option value="all">전체 작성자</option>
          {submitters.map(s => <option key={s} value={s}>{s}</option>)}
        </select>
        {conflicts.length > 0 && (
          <div className="text-xs text-red-600 font-medium">⚠️ 충돌 일정 {conflicts.length}건</div>
        )}
      </div>

      {/* Entry list grouped by submitter */}
      <div className="space-y-5">
        {Object.entries(groupedBySubmitter).map(([submitter, entries]) => (
          <div key={submitter} className="border border-gray-200 bg-white" style={{ borderRadius: 0 }}>
            <div className="bg-gray-50 border-b border-gray-200 px-4 py-2 flex items-center justify-between">
              <div className="flex items-center gap-3">
                <span className="text-sm font-medium text-gray-700">{submitter}</span>
                <span className="text-[10px] text-gray-500">{entries.length}건</span>
              </div>
              <div className="text-[10px] text-gray-400">
                확정: {entries.filter(e => e.status === 'confirmed').length} / 대기: {entries.filter(e => e.status === 'submitted').length}
              </div>
            </div>
            <div className="divide-y divide-gray-100">
              {entries.map(e => {
                const isPending = e.status === 'submitted'
                const isConflict = conflicts.some(c => c.id === e.id)
                return (
                  <div key={e.id} className={`px-4 py-3 ${isConflict ? 'bg-red-50' : isPending ? 'bg-yellow-50/50' : ''}`}>
                    <div className="flex items-start justify-between gap-3">
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2 mb-1">
                          <span className="text-xs font-medium text-gray-800">{e.content}</span>
                          <span className={`text-[10px] px-1.5 py-0.5 font-medium border
                            ${e.status === 'confirmed' ? 'bg-green-50 text-green-700 border-green-200' :
                              e.status === 'rejected' ? 'bg-red-50 text-red-600 border-red-200' :
                              e.status === 'submitted' ? 'bg-yellow-50 text-yellow-700 border-yellow-200' :
                              'bg-gray-50 text-gray-500 border-gray-200'}`}
                            style={{ borderRadius: 0 }}>
                            {entryStatusLabel(e.status)}
                          </span>
                          {isConflict && <span className="text-[10px] text-red-500 font-medium">⚠️ 중복</span>}
                        </div>
                        <div className="flex flex-wrap gap-x-3 gap-y-0.5 text-[10px] text-gray-500">
                          <span>📅 {new Date(e.planDate).toLocaleDateString('ko-KR')} ({dayOfWeekLabel(new Date(e.planDate).getDay(), true)})</span>
                          {e.target && <span>대상: {e.target}</span>}
                          {e.location && <span>장소: {e.location}</span>}
                          {e.personInCharge && <span>담당: {e.personInCharge}</span>}
                        </div>
                        {e.notes && <div className="text-[10px] text-gray-400 mt-1">📝 {e.notes}</div>}
                      </div>
                      {isPending && (
                        <div className="flex items-center gap-1.5 shrink-0">
                          <button onClick={() => updateEntryStatus(e.id, 'confirmed')}
                            className="px-3 py-1 text-[10px] font-medium text-white bg-green-700 hover:bg-green-800"
                            style={{ borderRadius: 0 }}>
                            확정
                          </button>
                          <button onClick={() => updateEntryStatus(e.id, 'rejected')}
                            className="px-3 py-1 text-[10px] font-medium text-red-600 bg-white border border-red-200 hover:bg-red-50"
                            style={{ borderRadius: 0 }}>
                            반려
                          </button>
                        </div>
                      )}
                      {!isPending && (
                        <div className="text-[10px] text-gray-400 shrink-0">
                          {e.status === 'confirmed' ? '✅' : e.status === 'rejected' ? '⛔' : ''}
                        </div>
                      )}
                    </div>
                  </div>
                )
              })}
            </div>
          </div>
        ))}

        {Object.keys(groupedBySubmitter).length === 0 && (
          <div className="border border-gray-200 bg-white p-8 text-center text-sm text-gray-400" style={{ borderRadius: 0 }}>
            조건에 맞는 일정이 없습니다
          </div>
        )}
      </div>
    </div>
  )
}
