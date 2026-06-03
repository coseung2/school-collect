'use client'

import { useState, useEffect } from 'react'
import { useParams } from 'next/navigation'
import Link from 'next/link'

const EVENT_TYPES = [
  { value: 'holiday', label: '공휴일' },
  { value: 'event', label: '행사' },
  { value: 'break', label: '방학' },
  { value: 'exam', label: '시험' },
  { value: 'etc', label: '기타' },
]

const MONTHS = [3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 1, 2]

export default function CalendarPage() {
  const params = useParams()
  const yearId = params.id as string

  const [events, setEvents] = useState<any[]>([])
  const [loading, setLoading] = useState(true)
  const [showForm, setShowForm] = useState(false)

  // 새 일정 폼
  const [newDate, setNewDate] = useState('')
  const [newType, setNewType] = useState('event')
  const [newTitle, setNewTitle] = useState('')
  const [newDesc, setNewDesc] = useState('')
  const [newIsSchoolDay, setNewIsSchoolDay] = useState(false)
  const [newAffectsTt, setNewAffectsTt] = useState(false)

  function load() {
    setLoading(true)
    fetch(`/api/curriculum/calendar-events?yearId=${yearId}`)
      .then(r => r.json())
      .then(d => setEvents(Array.isArray(d) ? d : []))
      .finally(() => setLoading(false))
  }

  useEffect(load, [yearId])

  async function addEvent() {
    if (!newDate || !newTitle) return
    const res = await fetch('/api/curriculum/calendar-events', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        year_id: yearId,
        date: newDate,
        type: newType,
        title: newTitle,
        description: newDesc || null,
        is_school_day: newIsSchoolDay,
        affects_timetable: newAffectsTt,
      }),
    })
    if (res.ok) {
      setNewDate('')
      setNewTitle('')
      setNewDesc('')
      setShowForm(false)
      load()
    }
  }

  async function removeEvent(id: string) {
    await fetch(`/api/curriculum/calendar-events?id=${id}`, { method: 'DELETE' })
    load()
  }

  const groupedByMonth: Record<number, any[]> = {}
  for (const e of events) {
    const m = new Date(e.date).getMonth() + 1
    if (!groupedByMonth[m]) groupedByMonth[m] = []
    groupedByMonth[m].push(e)
  }

  function monthLabel(m: number) {
    if (m >= 3) return `2026년 ${m}월`
    return `2027년 ${m}월`
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <div>
          <Link href="/admin/curriculum" className="text-sm text-warm-400 hover:underline">← 교육과정</Link>
          <h1 className="text-xl font-bold text-warm-800">📅 학사일정</h1>
        </div>
        <button onClick={() => setShowForm(!showForm)}
          className="btn btn-sm border border-warm-300 text-warm-600 hover:bg-warm-50">
          {showForm ? '✕ 닫기' : '+ 일정 추가'}
        </button>
      </div>

      {showForm && (
        <div className="card mb-4">
          <div className="text-sm font-bold text-warm-700 mb-2">새 일정 추가</div>
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 mb-2">
            <div>
              <div className="text-xs text-warm-400 mb-1">날짜</div>
              <input type="date" value={newDate} onChange={e => setNewDate(e.target.value)}
                className="border rounded px-2 py-1 text-sm w-full" />
            </div>
            <div>
              <div className="text-xs text-warm-400 mb-1">유형</div>
              <select value={newType} onChange={e => setNewType(e.target.value)}
                className="border rounded px-2 py-1 text-sm w-full">
                {EVENT_TYPES.map(t => <option key={t.value} value={t.value}>{t.label}</option>)}
              </select>
            </div>
            <div className="col-span-2">
              <div className="text-xs text-warm-400 mb-1">제목</div>
              <input value={newTitle} onChange={e => setNewTitle(e.target.value)}
                className="border rounded px-2 py-1 text-sm w-full" placeholder="개학식 / 현장체험학습" />
            </div>
          </div>
          <div>
            <div className="text-xs text-warm-400 mb-1">설명 (선택)</div>
            <input value={newDesc} onChange={e => setNewDesc(e.target.value)}
              className="border rounded px-2 py-1 text-sm w-full mb-2" placeholder="간단한 설명" />
          </div>
          <div className="flex gap-4 items-center mb-3">
            <label className="flex items-center gap-1 text-sm text-warm-600">
              <input type="checkbox" checked={newIsSchoolDay} onChange={e => setNewIsSchoolDay(e.target.checked)} />
              등교일
            </label>
            <label className="flex items-center gap-1 text-sm text-warm-600">
              <input type="checkbox" checked={newAffectsTt} onChange={e => setNewAffectsTt(e.target.checked)} />
              시간표 영향
            </label>
          </div>
          <button onClick={addEvent} className="btn btn-primary text-sm">추가</button>
        </div>
      )}

      {loading ? (
        <div className="card text-center py-8 text-warm-400">로딩중...</div>
      ) : events.length === 0 ? (
        <div className="card text-center py-8 text-warm-400">등록된 일정이 없습니다</div>
      ) : (
        <div className="space-y-4">
          {MONTHS.map(m => {
            const monthEvents = groupedByMonth[m]
            if (!monthEvents || monthEvents.length === 0) return null
            // Sort by date
            monthEvents.sort((a, b) => a.date.localeCompare(b.date))
            return (
              <div key={m} className="card">
                <div className="text-sm font-bold text-warm-700 mb-2">{monthLabel(m)}</div>
                <div className="divide-y divide-warm-100">
                  {monthEvents.map(e => (
                    <div key={e.id} className="flex items-center gap-3 py-1.5">
                      <span className="text-xs font-mono text-warm-400 w-24">
                        {new Date(e.date).toLocaleDateString('ko-KR', { month: '2-digit', day: '2-digit', weekday: 'short' })}
                      </span>
                      <span className={`px-1.5 py-0.5 text-xs rounded ${
                        e.type === 'holiday' ? 'bg-red-100 text-red-600' :
                        e.type === 'break' ? 'bg-blue-100 text-blue-600' :
                        e.type === 'event' ? 'bg-green-100 text-green-600' :
                        'bg-warm-100 text-warm-500'
                      }`}>
                        {EVENT_TYPES.find(t => t.value === e.type)?.label || e.type}
                      </span>
                      <span className="flex-1 text-sm text-warm-700">{e.title}</span>
                      {!e.is_school_day && <span className="text-xs text-red-400">미등교</span>}
                      {e.affects_timetable && <span className="text-xs text-blue-400">시간표변경</span>}
                      <button onClick={() => removeEvent(e.id)}
                        className="text-xs text-red-400 hover:text-red-600">삭제</button>
                    </div>
                  ))}
                </div>
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}
