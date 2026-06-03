'use client'

import { useState, useEffect } from 'react'
import { useParams } from 'next/navigation'
import Link from 'next/link'
import type { Subject, ElectiveSubject } from '@/lib/types'

const GRADES = [3, 4, 5, 6]

export default function ElectivesPage() {
  const params = useParams()
  const yearId = params.id as string

  const [subjects, setSubjects] = useState<Subject[]>([])
  const [electives, setElectives] = useState<ElectiveSubject[]>([])
  const [loading, setLoading] = useState(true)

  const [newName, setNewName] = useState('')
  const [newGrade, setNewGrade] = useState(0)
  const [newHours, setNewHours] = useState(34)
  const [newSubjectId, setNewSubjectId] = useState('')

  useEffect(() => {
    fetch('/api/curriculum/subjects')
      .then(r => r.json())
      .then(d => setSubjects(Array.isArray(d) ? d : []))
  }, [])

  function loadElectives() {
    setLoading(true)
    fetch(`/api/curriculum/electives?yearId=${yearId}`)
      .then(r => r.json())
      .then(d => setElectives(Array.isArray(d) ? d : []))
      .finally(() => setLoading(false))
  }

  useEffect(loadElectives, [yearId])

  async function add() {
    if (!newName) return
    const res = await fetch('/api/curriculum/electives', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        year_id: yearId,
        name: newName,
        grade: newGrade || null,
        total_hours: newHours,
        subject_id: newSubjectId || null,
      }),
    })
    if (res.ok) {
      setNewName('')
      setNewGrade(0)
      setNewHours(34)
      setNewSubjectId('')
      loadElectives()
    }
  }

  async function remove(id: string) {
    await fetch(`/api/curriculum/electives?id=${id}`, { method: 'DELETE' })
    loadElectives()
  }

  return (
    <div>
      <div className="mb-4">
        <Link href="/admin/curriculum" className="text-sm text-warm-400 hover:underline">← 교육과정</Link>
        <h1 className="text-xl font-bold text-warm-800">📚 학교자율과목</h1>
      </div>

      {/* 추가 폼 */}
      <div className="card mb-4">
        <div className="text-sm font-bold text-warm-700 mb-2">자율과목 추가</div>
        <div className="flex gap-2 items-end flex-wrap">
          <div>
            <div className="text-xs text-warm-400 mb-1">과목명</div>
            <input value={newName} onChange={e => setNewName(e.target.value)}
              className="border rounded px-2 py-1 text-sm w-36" placeholder="예: 코딩" />
          </div>
          <div>
            <div className="text-xs text-warm-400 mb-1">대상학년</div>
            <select value={newGrade} onChange={e => setNewGrade(+e.target.value)}
              className="border rounded px-2 py-1 text-sm">
              <option value={0}>전체</option>
              {GRADES.map(g => <option key={g} value={g}>{g}학년</option>)}
            </select>
          </div>
          <div>
            <div className="text-xs text-warm-400 mb-1">시수</div>
            <input type="number" value={newHours} onChange={e => setNewHours(+e.target.value)}
              className="border rounded px-2 py-1 text-sm w-16" />
          </div>
          <div>
            <div className="text-xs text-warm-400 mb-1">연계교과</div>
            <select value={newSubjectId} onChange={e => setNewSubjectId(e.target.value)}
              className="border rounded px-2 py-1 text-sm">
              <option value="">없음</option>
              {subjects.filter(s => s.category === 'general').map(s => (
                <option key={s.id} value={s.id}>{s.name}</option>
              ))}
            </select>
          </div>
          <button onClick={add} className="btn btn-primary text-sm">추가</button>
        </div>
      </div>

      {loading ? (
        <div className="card text-center py-8 text-warm-400">로딩중...</div>
      ) : electives.length === 0 ? (
        <div className="card text-center py-8 text-warm-400">등록된 자율과목이 없습니다</div>
      ) : (
        <div className="grid gap-2">
          {electives.map(e => (
            <div key={e.id} className="card flex items-center justify-between">
              <div className="flex items-center gap-3">
                <span className="font-medium text-warm-800">{e.name}</span>
                <span className="text-xs text-warm-400">
                  {e.grade ? `${e.grade}학년` : '전체'}
                </span>
                <span className="text-xs px-1.5 py-0.5 rounded bg-blue-50 text-blue-600">
                  {e.totalHours}차시
                </span>
                {e.subjectId && (
                  <span className="text-xs text-warm-400">
                    연계: {subjects.find(s => s.id === e.subjectId)?.name || '-'}
                  </span>
                )}
              </div>
              <button onClick={() => remove(e.id)}
                className="text-xs text-red-400 hover:text-red-600">삭제</button>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
