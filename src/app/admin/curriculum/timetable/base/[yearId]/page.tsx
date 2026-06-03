'use client'

import { useState, useEffect, useCallback } from 'react'
import { useParams } from 'next/navigation'
import Link from 'next/link'

const DAYS = ['월', '화', '수', '목', '금']
const PERIODS = [1, 2, 3, 4, 5, 6, 7]
const GRADES = [1, 2, 3, 4, 5, 6]
const CLASSES = [1, 2, 3, 4, 5, 6]

export default function BaseTimetablePage() {
  const params = useParams()
  const yearId = params.yearId as string

  const [subjects, setSubjects] = useState<{ id: string; name: string }[]>([])
  const [grade, setGrade] = useState(1)
  const [classNo, setClassNo] = useState(1)
  const [grid, setGrid] = useState<Record<string, string>>({})
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [msg, setMsg] = useState('')

  useEffect(() => {
    fetch('/api/curriculum/subjects').then(r => r.json()).then(d => setSubjects(Array.isArray(d) ? d : [])).catch(() => {})
  }, [])

  const loadGrid = useCallback(async () => {
    setLoading(true)
    const url = `/api/curriculum/timetables?yearId=${yearId}&sourceType=base&grade=${grade}&classNo=${classNo}`
    const res = await fetch(url)
    const data = await res.json()
    const g: Record<string, string> = {}
    for (const e of Array.isArray(data) ? data : []) {
      g[`${e.weekday}-${e.period}`] = e.subject_id || ''
    }
    setGrid(g)
    setLoading(false)
  }, [yearId, grade, classNo])

  useEffect(() => { loadGrid() }, [loadGrid])

  async function save() {
    setSaving(true)
    setMsg('')
    const entries: any[] = []
    for (const [key, subjectId] of Object.entries(grid)) {
      if (!subjectId) continue
      const [weekday, period] = key.split('-').map(Number)
      entries.push({ year_id: yearId, grade, class_no: classNo, weekday, period, subject_id: subjectId })
    }
    const res = await fetch('/api/curriculum/timetables/bulk', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ sourceType: 'base', yearId, grade, classNo, entries: entries.map(e => ({ weekday: e.weekday, period: e.period, subjectId: e.subject_id })) }),
    })
    if (res.ok) setMsg('✅ 저장 완료')
    else setMsg('❌ 저장 실패')
    setSaving(false)
    setTimeout(() => setMsg(''), 2000)
  }

  function cycleSubject(weekday: number, period: number) {
    const key = `${weekday}-${period}`
    const cur = grid[key] || ''
    const idx = subjects.findIndex(s => s.id === cur)
    const next = subjects[(idx + 1) % subjects.length]
    setGrid(prev => ({ ...prev, [key]: next.id }))
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <div>
          <Link href="/admin/curriculum" className="text-sm text-warm-400 hover:underline">← 교육과정</Link>
          <h1 className="text-xl font-bold text-warm-800">기초시간표</h1>
        </div>
        <button onClick={save} disabled={saving} className="btn btn-primary">
          {saving ? '저장중...' : '💾 저장'}
        </button>
      </div>

      <div className="flex gap-4 mb-4 items-center flex-wrap">
        <label className="text-sm text-warm-600">학년
          <select value={grade} onChange={e => setGrade(+e.target.value)} className="ml-2 border rounded px-2 py-1 text-sm">
            {GRADES.map(g => <option key={g} value={g}>{g}학년</option>)}
          </select>
        </label>
        <label className="text-sm text-warm-600">반
          <select value={classNo} onChange={e => setClassNo(+e.target.value)} className="ml-2 border rounded px-2 py-1 text-sm">
            {CLASSES.map(c => <option key={c} value={c}>{c}반</option>)}
          </select>
        </label>
        <button onClick={() => { setGrid({}); setMsg('초기화됨') }}
          className="text-xs px-2 py-1 border border-red-300 text-red-500 rounded hover:bg-red-50">✕ 초기화</button>
        {msg && <span className="text-sm text-green-600 font-medium">{msg}</span>}
      </div>

      {loading ? (
        <div className="card text-center py-8 text-warm-400">로딩중...</div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full border-collapse text-sm">
            <thead>
              <tr>
                <th className="border p-2 bg-warm-50 text-warm-500 w-12">교시</th>
                {DAYS.map(d => <th key={d} className="border p-2 bg-warm-50 text-warm-600">{d}</th>)}
              </tr>
            </thead>
            <tbody>
              {PERIODS.map(p => (
                <tr key={p}>
                  <td className="border p-2 text-center text-warm-400 font-bold">{p}</td>
                  {[1,2,3,4,5].map(wd => {
                    const sid = grid[`${wd}-${p}`] || ''
                    const sub = subjects.find(s => s.id === sid)
                    return (
                      <td key={wd} onClick={() => cycleSubject(wd, p)}
                        className={`border p-2 text-center cursor-pointer transition text-xs min-w-[60px] h-9
                          ${sid ? 'bg-blue-50 text-blue-700 font-medium' : 'text-warm-300 hover:bg-warm-50'}`}>
                        {sub?.name || '＋'}
                      </td>
                    )
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}