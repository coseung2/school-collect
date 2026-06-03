'use client'

import { useState, useEffect } from 'react'
import { useParams } from 'next/navigation'
import Link from 'next/link'

export default function CrossTopicsPage() {
  const params = useParams()
  const yearId = params.yearId as string

  const [topics, setTopics] = useState<any[]>([])
  const [loading, setLoading] = useState(true)
  const [newName, setNewName] = useState('')
  const [newCategory, setNewCategory] = useState('safety')
  const [newHours, setNewHours] = useState(0)

  const CATEGORIES = [
    { value: 'safety', label: '안전교육' },
    { value: 'character', label: '인성교육' },
    { value: 'environment', label: '환경교육' },
    { value: 'career', label: '진로교육' },
    { value: 'democracy', label: '민주시민' },
    { value: 'reading', label: '독서교육' },
    { value: 'sex', label: '성교육' },
    { value: 'multicultural', label: '다문화' },
    { value: 'economics', label: '경제교육' },
    { value: 'other', label: '기타' },
  ]

  useEffect(() => {
    setLoading(true)
    fetch(`/api/curriculum/cross-topics?yearId=${yearId}`)
      .then(r => r.json())
      .then(d => setTopics(Array.isArray(d) ? d : []))
      .finally(() => setLoading(false))
  }, [yearId])

  async function addTopic() {
    if (!newName) return
    const res = await fetch('/api/curriculum/cross-topics', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        year_id: yearId,
        name: newName,
        category: newCategory,
        default_hours: newHours,
      }),
    })
    if (res.ok) {
      setNewName('')
      setNewHours(0)
      const data = await res.json()
      setTopics(prev => [...prev, data].sort((a, b) => a.name.localeCompare(b.name)))
    }
  }

  return (
    <div>
      <div className="mb-4">
        <Link href="/admin/curriculum" className="text-sm text-warm-400 hover:underline">← 교육과정</Link>
        <h1 className="text-xl font-bold text-warm-800">🏷️ 범교과주제</h1>
      </div>

      {/* 새 주제 추가 */}
      <div className="card mb-4">
        <div className="text-sm font-bold text-warm-700 mb-2">새 범교과주제 추가</div>
        <div className="flex gap-2 items-end flex-wrap">
          <div>
            <div className="text-xs text-warm-400 mb-1">주제명</div>
            <input value={newName} onChange={e => setNewName(e.target.value)}
              className="border rounded px-2 py-1 text-sm w-40" placeholder="안전교육" />
          </div>
          <div>
            <div className="text-xs text-warm-400 mb-1">분류</div>
            <select value={newCategory} onChange={e => setNewCategory(e.target.value)}
              className="border rounded px-2 py-1 text-sm">
              {CATEGORIES.map(c => <option key={c.value} value={c.value}>{c.label}</option>)}
            </select>
          </div>
          <div>
            <div className="text-xs text-warm-400 mb-1">권장시수</div>
            <input type="number" value={newHours} onChange={e => setNewHours(+e.target.value)}
              className="border rounded px-2 py-1 text-sm w-16" />
          </div>
          <button onClick={addTopic} className="btn btn-primary text-sm">추가</button>
        </div>
      </div>

      {loading ? (
        <div className="card text-center py-8 text-warm-400">로딩중...</div>
      ) : topics.length === 0 ? (
        <div className="card text-center py-8 text-warm-400">등록된 주제가 없습니다</div>
      ) : (
        <div className="grid gap-2">
          {topics.map(t => (
            <div key={t.id} className="card flex items-center justify-between">
              <div>
                <span className="font-medium text-warm-800">{t.name}</span>
                <span className="ml-2 px-1.5 py-0.5 text-xs rounded bg-warm-100 text-warm-500">
                  {CATEGORIES.find(c => c.value === t.category)?.label || t.category}
                </span>
              </div>
              <div className="text-sm text-warm-400">권장 {t.default_hours}차시</div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}