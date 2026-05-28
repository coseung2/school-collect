'use client'

import { useState } from 'react'
import { useRouter } from 'next/navigation'

export default function NewMonthlyPlanPage() {
  const router = useRouter()
  const [year, setYear] = useState(new Date().getFullYear().toString())
  const [month, setMonth] = useState((new Date().getMonth() + 1).toString())
  const [title, setTitle] = useState('')
  const [description, setDescription] = useState('')
  const [deadline, setDeadline] = useState('')
  const [saving, setSaving] = useState(false)

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    setSaving(true)
    try {
      const res = await fetch('/api/plans', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          year: parseInt(year),
          month: parseInt(month),
          title,
          description,
          deadline: deadline ? new Date(deadline).toISOString() : null,
        }),
      })
      if (!res.ok) throw new Error(await res.text())
      const plan = await res.json()
      router.push(`/plan/${plan.id}`)
    } catch (err: any) {
      alert('생성 실패: ' + err.message)
    } finally {
      setSaving(false)
    }
  }

  return (
    <div className="max-w-xl mx-auto">
      <h1 className="text-lg font-bold text-gray-800 mb-6">새 월중계획 등록</h1>

      <form onSubmit={handleSubmit} className="border border-gray-200 bg-white rounded-none">
        <div className="p-6 space-y-5">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-xs font-medium text-gray-600 mb-1">연도</label>
              <select className="w-full border border-gray-200 px-3 py-2 text-sm bg-white rounded-none"
                value={year} onChange={e => setYear(e.target.value)}>
                {[2025, 2026, 2027].map(y => (
                  <option key={y} value={y}>{y}년</option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-xs font-medium text-gray-600 mb-1">월</label>
              <select className="w-full border border-gray-200 px-3 py-2 text-sm bg-white rounded-none"
                value={month} onChange={e => setMonth(e.target.value)}>
                {Array.from({ length: 12 }, (_, i) => i + 1).map(m => (
                  <option key={m} value={m}>{m}월</option>
                ))}
              </select>
            </div>
          </div>

          <div>
            <label className="block text-xs font-medium text-gray-600 mb-1">제목</label>
            <input className="w-full border border-gray-200 px-3 py-2 text-sm rounded-none"
              value={title} onChange={e => setTitle(e.target.value)}
              placeholder="예: 4월 월중계획" required />
          </div>

          <div>
            <label className="block text-xs font-medium text-gray-600 mb-1">설명 (선택)</label>
            <textarea className="w-full border border-gray-200 px-3 py-2 text-sm rounded-none" rows={2}
              value={description} onChange={e => setDescription(e.target.value)}
              placeholder="간단한 안내" />
          </div>

          <div>
            <label className="block text-xs font-medium text-gray-600 mb-1">제출 마감일 (선택)</label>
            <input className="w-full border border-gray-200 px-3 py-2 text-sm rounded-none" type="date"
              value={deadline} onChange={e => setDeadline(e.target.value)} />
          </div>
        </div>

        <div className="border-t border-gray-200 px-6 py-3 flex gap-3 bg-gray-50">
          <button type="submit" disabled={saving}
            className="px-5 py-2 text-sm font-medium text-white bg-gray-800 hover:bg-gray-900 rounded-none">
            {saving ? '저장중...' : '월중계획 등록'}
          </button>
          <button type="button" onClick={() => router.back()}
            className="px-5 py-2 text-sm font-medium text-gray-600 bg-gray-100 hover:bg-gray-200 rounded-none">
            취소
          </button>
        </div>
      </form>
    </div>
  )
}
