'use client'

import { useState, useEffect } from 'react'
import Link from 'next/link'
import type { CurriculumYear } from '@/lib/types'

const STATUS_BADGE: Record<string, string> = {
  draft: 'bg-gray-100 text-gray-600',
  editing: 'bg-blue-100 text-blue-700',
  validated: 'bg-green-100 text-green-700',
  confirmed: 'bg-purple-100 text-purple-700',
  archived: 'bg-warm-100 text-warm-500',
}

const STEPS = [
  { key: 'classes', label: '학년/반 설정', path: (id: string) => `/admin/curriculum/classes/${id}` },
  { key: 'hours', label: '시수편제', path: (id: string) => `/admin/curriculum/hours/${id}` },
  { key: 'calendar', label: '학사일정', path: (id: string) => `/admin/curriculum/calendar/${id}` },
  { key: 'timetable_base', label: '기초시간표', path: (id: string) => `/admin/curriculum/timetable/base/${id}` },
  { key: 'timetable_semester', label: '학기별 시간표', path: (id: string) => `/admin/curriculum/timetable/semester/${id}` },
  { key: 'lessons', label: '교과 진도표', path: (id: string) => `/admin/curriculum/lessons/${id}` },
  { key: 'cross_topics', label: '범교과주제', path: (id: string) => `/admin/curriculum/cross-topics/${id}` },
  { key: 'electives', label: '학교자율과목', path: (id: string) => `/admin/curriculum/electives/${id}` },
  { key: 'validate', label: '검증', path: (id: string) => `/admin/curriculum/validate/${id}` },
]

export default function CurriculumPage() {
  const [years, setYears] = useState<CurriculumYear[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    fetch('/api/curriculum/years?schoolId=3fd5c391-a698-46da-a8b0-a67959b498de')
      .then(r => r.json())
      .then(d => setYears(Array.isArray(d) ? d : []))
      .catch(() => setYears([]))
      .finally(() => setLoading(false))
  }, [])

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-bold text-warm-800">📋 교육과정 작성</h1>
        <Link href="/admin/curriculum/years/new" className="btn btn-primary text-sm">+ 새 학년도</Link>
      </div>

      {loading ? (
        <div className="card text-center py-8 text-warm-400">로딩중...</div>
      ) : years.length === 0 ? (
        <div className="card text-center py-12 text-warm-400">
          <div className="text-3xl mb-2">📅</div>
          <div className="text-sm mb-4">등록된 교육과정이 없습니다</div>
          <Link href="/admin/curriculum/years/new" className="btn btn-primary inline-block">새 학년도 만들기</Link>
        </div>
      ) : (
        <div className="grid gap-4">
          {years.map(y => (
            <div key={y.id} className="card">
              <div className="flex items-center justify-between mb-3">
                <div>
                  <span className="text-lg font-bold text-warm-800">{y.year}학년도</span>
                  <span className={`ml-2 px-2 py-0.5 text-xs rounded ${STATUS_BADGE[y.status] || ''}`}>
                    {y.status === 'draft' ? '작성중' : y.status === 'editing' ? '편집중' : y.status === 'validated' ? '검증완료' : y.status === 'confirmed' ? '확정' : '보관'}
                  </span>
                </div>
              </div>
              <div className="grid grid-cols-3 sm:grid-cols-5 gap-2">
                {STEPS.map(s => (
                  <Link key={s.key} href={s.path(y.id)}
                    className="text-center p-2 rounded border border-warm-200 text-sm text-warm-600 hover:bg-warm-50 transition">
                    {s.label}
                  </Link>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}