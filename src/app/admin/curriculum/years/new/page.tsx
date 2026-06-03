'use client'

import { useState } from 'react'
import { useRouter } from 'next/navigation'
import Link from 'next/link'

const SCHOOL_ID = '3fd5c391-a698-46da-a8b0-a67959b498de'

export default function NewCurriculumYearPage() {
  const router = useRouter()
  const [year, setYear] = useState(2026)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  async function create() {
    setLoading(true)
    setError('')
    const res = await fetch('/api/curriculum/years', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ school_id: SCHOOL_ID, year, status: 'draft', is_active: true }),
    })
    const data = await res.json()
    if (!res.ok) {
      setError(data.error || '생성 실패')
      setLoading(false)
      return
    }
    router.push(`/admin/curriculum/timetable/base/${data.id}`)
  }

  return (
    <div className="max-w-md mx-auto mt-12">
      <Link href="/admin/curriculum" className="text-sm text-warm-400 hover:underline">← 교육과정</Link>
      <h1 className="text-xl font-bold text-warm-800 mt-2 mb-6">📅 새 학년도 만들기</h1>

      <div className="card">
        <label className="text-sm text-warm-600 block mb-1">학년도</label>
        <input type="number" value={year} onChange={e => setYear(+e.target.value)}
          className="border rounded px-3 py-2 text-lg w-full mb-4" min={2024} max={2030} />

        <div className="text-xs text-warm-400 mb-4">
          장량초등학교 — 2026학년도 1학기/2학기
        </div>

        {error && <div className="text-red-500 text-sm mb-3">{error}</div>}

        <button onClick={create} disabled={loading}
          className="btn btn-primary w-full text-center">
          {loading ? '생성중...' : '📋 학년도 생성하고 기초시간표로 이동'}
        </button>
      </div>
    </div>
  )
}
