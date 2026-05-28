'use client'

import { useState, useEffect } from 'react'
import { useParams } from 'next/navigation'
import { entryStatusLabel, monthLabel, dayOfWeekLabel } from '@/lib/types'
import type { MonthlyPlan, MonthlyPlanEntry } from '@/lib/types'

export default function PlanViewPage() {
  const { planId } = useParams<{ planId: string }>()
  const [plan, setPlan] = useState<MonthlyPlan | null>(null)
  const [entries, setEntries] = useState<MonthlyPlanEntry[]>([])
  const [loading, setLoading] = useState(true)
  const [viewMode, setViewMode] = useState<'month' | 'week' | 'day'>('week')
  const [selectedDate, setSelectedDate] = useState<Date>(new Date())

  useEffect(() => {
    Promise.all([
      fetch(`/api/plans?id=${planId}`).then(r => r.json()),
      fetch(`/api/entries?planId=${planId}`).then(r => r.json()),
    ]).then(([planData, entryData]) => {
      setPlan(planData)
      setEntries(entryData || [])
      setLoading(false)
    })
  }, [planId])

  if (loading) return <div className="p-6 text-gray-400 text-sm">로딩중...</div>
  if (!plan) return <div className="p-6 text-gray-400 text-sm">월중계획을 찾을 수 없습니다</div>

  const confirmed = entries.filter(e => e.status === 'confirmed')
  const startDate = new Date(plan.year, plan.month - 1, 1)
  const endDate = new Date(plan.year, plan.month, 0)
  const daysInMonth = endDate.getDate()
  const firstDayOfWeek = startDate.getDay()

  const getDaysForWeek = (weekIndex: number) => {
    const days: Date[] = []
    const start = weekIndex * 7 - firstDayOfWeek
    for (let i = 0; i < 7; i++) {
      const d = new Date(plan.year, plan.month - 1, start + i)
      if (d.getMonth() === plan.month - 1) days.push(d)
    }
    return days
  }

  const weekCount = Math.ceil((daysInMonth + firstDayOfWeek) / 7)
  const weeks = Array.from({ length: weekCount }, (_, i) => i)

  const todayStr = new Date().toISOString().slice(0, 10)
  const currentWeekIndex = weeks.findIndex(w => {
    const days = getDaysForWeek(w)
    return days.some(d => d.toISOString().slice(0, 10) === todayStr)
  })

  const [activeWeek, setActiveWeek] = useState(currentWeekIndex >= 0 ? currentWeekIndex : 0)

  const getEntriesForDate = (dateStr: string) =>
    confirmed.filter(e => e.planDate.slice(0, 10) === dateStr)

  const getEntriesForWeek = (weekIdx: number) => {
    const days = getDaysForWeek(weekIdx)
    const dateStrs = days.map(d => d.toISOString().slice(0, 10))
    return confirmed.filter(e => dateStrs.includes(e.planDate.slice(0, 10)))
  }

  const selectedDateStr = selectedDate.toISOString().slice(0, 10)
  const dayEntries = getEntriesForDate(selectedDateStr)

  return (
    <div style={{ maxWidth: 1100, margin: '0 auto' }}>
      {/* Header */}
      <div className="mb-5 pb-4 border-b border-gray-200">
        <div className="flex items-center justify-between mb-2">
          <h1 className="text-lg font-bold text-gray-800">
            {plan.title || `${plan.year}년 ${plan.month}월 월중계획`}
          </h1>
          <div className="flex items-center gap-2 text-xs text-gray-500">
            <span>총 {confirmed.length}건</span>
          </div>
        </div>
        {plan.description && (
          <p className="text-sm text-gray-500 mb-3">{plan.description}</p>
        )}
      </div>

      {/* View Tabs + Today Button */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex border border-gray-200 divide-x divide-gray-200" style={{ borderRadius: 0 }}>
          {(['month', 'week', 'day'] as const).map(mode => (
            <button key={mode}
              onClick={() => setViewMode(mode)}
              className={`px-4 py-2 text-xs font-medium transition-colors
                ${viewMode === mode ? 'bg-gray-800 text-white' : 'bg-white text-gray-600 hover:bg-gray-50'}`}
              style={{ borderRadius: 0 }}>
              {mode === 'month' ? '월 보기' : mode === 'week' ? '주 보기' : '일 보기'}
            </button>
          ))}
        </div>
        {viewMode === 'week' && (
          <button onClick={() => setActiveWeek(currentWeekIndex >= 0 ? currentWeekIndex : 0)}
            className="px-3 py-2 text-xs font-medium bg-gray-100 text-gray-600 hover:bg-gray-200 border border-gray-200"
            style={{ borderRadius: 0 }}>
            이번 주
          </button>
        )}
      </div>

      {/* ─── MONTH VIEW ─── */}
      {viewMode === 'month' && (
        <div className="border border-gray-200 bg-white" style={{ borderRadius: 0 }}>
          <div className="grid grid-cols-7 text-center">
            {['일', '월', '화', '수', '목', '금', '토'].map(d => (
              <div key={d} className="py-2 text-xs font-medium text-gray-500 border-b border-gray-200 bg-gray-50">
                {d}
              </div>
            ))}
          </div>
          <div className="grid grid-cols-7 text-left">
            {Array.from({ length: firstDayOfWeek }, (_, i) => (
              <div key={`empty-${i}`} className="min-h-[90px] bg-gray-50 border-b border-r border-gray-100" />
            ))}
            {Array.from({ length: daysInMonth }, (_, i) => {
              const day = i + 1
              const dateStr = `${plan.year}-${String(plan.month).padStart(2, '0')}-${String(day).padStart(2, '0')}`
              const dayEntries = getEntriesForDate(dateStr)
              const dow = new Date(plan.year, plan.month - 1, day).getDay()
              const isToday = dateStr === todayStr
              return (
                <div key={day}
                  className={`min-h-[90px] p-1.5 border-b border-r border-gray-100 text-xs
                    ${isToday ? 'bg-yellow-50' : ''}`}>
                  <div className={`font-medium mb-1 ${isToday ? 'text-blue-700' : dow === 0 ? 'text-red-400' : 'text-gray-700'}`}>
                    {day}
                  </div>
                  <div className="space-y-0.5">
                    {dayEntries.slice(0, 3).map(e => (
                      <div key={e.id} className="text-[10px] leading-tight text-gray-600 truncate bg-gray-50 px-1 py-0.5"
                        title={e.content}>
                        {e.content}
                      </div>
                    ))}
                    {dayEntries.length > 3 && (
                      <div className="text-[10px] text-gray-400">+{dayEntries.length - 3}개</div>
                    )}
                  </div>
                </div>
              )
            })}
          </div>
        </div>
      )}

      {/* ─── WEEK VIEW ─── */}
      {viewMode === 'week' && (
        <div>
          {/* Week navigation */}
          <div className="flex items-center gap-2 mb-3 overflow-x-auto border-b border-gray-200 pb-2">
            {weeks.map(w => {
              const days = getDaysForWeek(w)
              const label = `${days[0]?.getDate() || ''}${days.length > 1 ? `~${days[days.length - 1]?.getDate()}` : ''}`
              const isActive = w === activeWeek
              return (
                <button key={w}
                  onClick={() => setActiveWeek(w)}
                  className={`text-xs px-3 py-1.5 whitespace-nowrap border border-gray-200
                    ${isActive ? 'bg-gray-800 text-white' : 'bg-white text-gray-600 hover:bg-gray-50'}`}
                  style={{ borderRadius: 0 }}>
                  {label}일 주
                </button>
              )
            })}
          </div>

          {/* Week table */}
          <div className="border border-gray-200 bg-white" style={{ borderRadius: 0 }}>
            <div className="grid grid-cols-7 text-center">
              {['일', '월', '화', '수', '목', '금', '토'].map(d => (
                <div key={d} className="py-2 text-xs font-medium text-gray-500 border-b border-gray-200 bg-gray-50">
                  {d}
                </div>
              ))}
            </div>
            <div className="grid grid-cols-7 text-left">
              {getDaysForWeek(activeWeek).map(d => {
                const dateStr = d.toISOString().slice(0, 10)
                const dayEntries = getEntriesForDate(dateStr)
                const dow = d.getDay()
                const isToday = dateStr === todayStr
                return (
                  <div key={dateStr}
                    className={`min-h-[120px] p-1.5 border-r border-b border-gray-100 text-xs
                      ${isToday ? 'bg-yellow-50' : ''}`}>
                    <div className={`font-medium mb-1 ${isToday ? 'text-blue-700' : dow === 0 ? 'text-red-400' : 'text-gray-700'}`}>
                      {d.getDate()}일
                    </div>
                    <div className="space-y-0.5">
                      {dayEntries.map(e => (
                        <div key={e.id}
                          className="text-[10px] leading-tight text-gray-600 px-1 py-0.5 border-l-2 border-gray-400 mb-0.5"
                          title={e.content}>
                          <div className="truncate font-medium">{e.content}</div>
                          {(e.target || e.location) && (
                            <div className="text-[9px] text-gray-400 truncate">
                              {e.target}{e.target && e.location ? ' · ' : ''}{e.location}
                            </div>
                          )}
                        </div>
                      ))}
                    </div>
                  </div>
                )
              })}
            </div>
          </div>
        </div>
      )}

      {/* ─── DAY VIEW ─── */}
      {viewMode === 'day' && (
        <div>
          {/* Date picker */}
          <div className="flex items-center gap-3 mb-4">
            <button onClick={() => {
              const prev = new Date(selectedDate)
              prev.setDate(prev.getDate() - 1)
              setSelectedDate(prev)
            }}
              className="px-3 py-1.5 text-xs border border-gray-200 bg-white hover:bg-gray-50"
              style={{ borderRadius: 0 }}>
              ◀ 전날
            </button>
            <input type="date" value={selectedDateStr}
              onChange={e => setSelectedDate(new Date(e.target.value))}
              className="border border-gray-200 px-3 py-1.5 text-xs bg-white" style={{ borderRadius: 0 }} />
            <button onClick={() => {
              const next = new Date(selectedDate)
              next.setDate(next.getDate() + 1)
              setSelectedDate(next)
            }}
              className="px-3 py-1.5 text-xs border border-gray-200 bg-white hover:bg-gray-50"
              style={{ borderRadius: 0 }}>
              다음날 ▶
            </button>
            <button onClick={() => setSelectedDate(new Date())}
              className="px-3 py-1.5 text-xs font-medium text-blue-600 border border-blue-200 bg-blue-50 hover:bg-blue-100"
              style={{ borderRadius: 0 }}>
              오늘
            </button>
          </div>

          {/* Day entries */}
          {dayEntries.length === 0 ? (
            <div className="border border-gray-200 bg-white p-8 text-center text-sm text-gray-400" style={{ borderRadius: 0 }}>
              이 날짜의 일정이 없습니다
            </div>
          ) : (
            <div className="border border-gray-200 bg-white divide-y divide-gray-100" style={{ borderRadius: 0 }}>
              {dayEntries.map(e => (
                <div key={e.id} className="p-4">
                  <div className="flex items-start justify-between mb-1">
                    <div className="text-sm font-medium text-gray-800">{e.content}</div>
                    <span className="text-[10px] text-gray-400 whitespace-nowrap ml-3">
                      {dayOfWeekLabel(new Date(e.planDate).getDay(), true)}요일
                    </span>
                  </div>
                  <div className="flex flex-wrap gap-3 text-xs text-gray-500">
                    {e.target && <span>대상: {e.target}</span>}
                    {e.location && <span>장소: {e.location}</span>}
                    {e.personInCharge && <span>담당: {e.personInCharge}</span>}
                  </div>
                  {e.notes && <div className="text-xs text-gray-400 mt-1">{e.notes}</div>}
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  )
}
