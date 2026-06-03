'use client'

import { useState, useEffect } from 'react'
import { useParams } from 'next/navigation'
import Link from 'next/link'

export default function LessonsPage() {
  const params = useParams()
  const yearId = params.yearId as string

  const [subjects, setSubjects] = useState<{ id: string; name: string }[]>([])
  const [lessons, setLessons] = useState<any[]>([])
  const [term, setTerm] = useState(1)
  const [grade, setGrade] = useState(1)
  const [classNo, setClassNo] = useState(1)
  const [subjectId, setSubjectId] = useState('')
  const [loading, setLoading] = useState(true)

  // 새 차시 입력
  const [newUnit, setNewUnit] = useState('')
  const [newLessonNo, setNewLessonNo] = useState(1)
  const [newContent, setNewContent] = useState('')

  useEffect(() => {
    fetch('/api/curriculum/subjects').then(r => r.json()).then(d => setSubjects(Array.isArray(d) ? d : [])).catch(() => {})
  }, [])

  function loadLessons() {
    if (!subjectId) return
    setLoading(true)
    const params = new URLSearchParams({ yearId, term: String(term), grade: String(grade), classNo: String(classNo), subjectId })
    fetch(`/api/curriculum/lessons?${params}`)
      .then(r => r.json())
      .then(d => setLessons(Array.isArray(d) ? d : []))
      .finally(() => setLoading(false))
  }

  useEffect(() => { loadLessons() }, [yearId, term, grade, classNo, subjectId])

  async function addLesson() {
    if (!newUnit || !newContent || !subjectId) return
    const res = await fetch('/api/curriculum/lessons', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        year_id: yearId,
        term,
        grade,
        class_no: classNo,
        subject_id: subjectId,
        unit: newUnit,
        lesson_no: newLessonNo,
        content: newContent,
      }),
    })
    if (res.ok) {
      setNewUnit('')
      setNewContent('')
      setNewLessonNo(prev => prev + 1)
      loadLessons()
    }
  }

  async function removeLesson(id: string) {
    await fetch(`/api/curriculum/lessons?id=${id}`, { method: 'DELETE' })
    loadLessons()
  }

  return (
    <div>
      <div className="mb-4">
        <Link href="/admin/curriculum" className="text-sm text-warm-400 hover:underline">← 교육과정</Link>
        <h1 className="text-xl font-bold text-warm-800">📖 교과 진도표</h1>
      </div>

      <div className="flex gap-4 mb-4 items-center flex-wrap">
        <label className="text-sm text-warm-600">학기
          <select value={term} onChange={e => setTerm(+e.target.value)} className="ml-2 border rounded px-2 py-1 text-sm">
            <option value={1}>1학기</option>
            <option value={2}>2학기</option>
          </select>
        </label>
        <label className="text-sm text-warm-600">학년
          <select value={grade} onChange={e => setGrade(+e.target.value)} className="ml-2 border rounded px-2 py-1 text-sm">
            {[1,2,3,4,5,6].map(g => <option key={g} value={g}>{g}학년</option>)}
          </select>
        </label>
        <label className="text-sm text-warm-600">반
          <select value={classNo} onChange={e => setClassNo(+e.target.value)} className="ml-2 border rounded px-2 py-1 text-sm">
            {[1,2,3,4,5,6].map(c => <option key={c} value={c}>{c}반</option>)}
          </select>
        </label>
        <label className="text-sm text-warm-600">교과
          <select value={subjectId} onChange={e => { setSubjectId(e.target.value); setNewLessonNo(1) }}
            className="ml-2 border rounded px-2 py-1 text-sm min-w-[100px]">
            <option value="">선택</option>
            {subjects.map(s => <option key={s.id} value={s.id}>{s.name}</option>)}
          </select>
        </label>
      </div>

      {subjectId ? (
        <>
          {/* 새 차시 입력 폼 */}
          <div className="card mb-4">
            <div className="text-sm font-bold text-warm-700 mb-2">새 차시 추가</div>
            <div className="flex gap-2 items-end flex-wrap">
              <div>
                <div className="text-xs text-warm-400 mb-1">단원</div>
                <input value={newUnit} onChange={e => setNewUnit(e.target.value)}
                  className="border rounded px-2 py-1 text-sm w-32" placeholder="1. 분수의 덧셈" />
              </div>
              <div>
                <div className="text-xs text-warm-400 mb-1">차시</div>
                <input type="number" value={newLessonNo} onChange={e => setNewLessonNo(+e.target.value)}
                  className="border rounded px-2 py-1 text-sm w-16" />
              </div>
              <div className="flex-1 min-w-[200px]">
                <div className="text-xs text-warm-400 mb-1">학습내용</div>
                <input value={newContent} onChange={e => setNewContent(e.target.value)}
                  className="border rounded px-2 py-1 text-sm w-full" placeholder="분수의 덧셈 원리 이해하기" />
              </div>
              <button onClick={addLesson} className="btn btn-primary text-sm">추가</button>
            </div>
          </div>

          {/* 차시 목록 */}
          {loading ? (
            <div className="card text-center py-8 text-warm-400">로딩중...</div>
          ) : lessons.length === 0 ? (
            <div className="card text-center py-8 text-warm-400">등록된 차시가 없습니다</div>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full border-collapse text-sm">
                <thead>
                  <tr>
                    <th className="border p-2 bg-warm-50 text-warm-500 w-12">차시</th>
                    <th className="border p-2 bg-warm-50 text-warm-600">단원</th>
                    <th className="border p-2 bg-warm-50 text-warm-600">학습내용</th>
                    <th className="border p-2 bg-warm-50 text-warm-500 w-20">관리</th>
                  </tr>
                </thead>
                <tbody>
                  {lessons.map(l => (
                    <tr key={l.id}>
                      <td className="border p-2 text-center text-warm-500 font-bold">{l.lesson_no}</td>
                      <td className="border p-2 text-warm-700">{l.unit}</td>
                      <td className="border p-2 text-warm-600">{l.content}</td>
                      <td className="border p-2 text-center">
                        <button onClick={() => removeLesson(l.id)}
                          className="text-xs text-red-500 hover:underline">삭제</button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </>
      ) : (
        <div className="card text-center py-8 text-warm-400">교과를 선택해주세요</div>
      )}
    </div>
  )
}