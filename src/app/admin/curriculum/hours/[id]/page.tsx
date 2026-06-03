'use client'

import { useState, useEffect } from 'react'
import { useParams } from 'next/navigation'
import Link from 'next/link'
import type { Subject, CurriculumHour } from '@/lib/types'

const GRADES = [1, 2, 3, 4, 5, 6]

// 학년별 권장시수 (2022 개정 교육과정 기준, 초등)
const DEFAULT_HOURS: Record<number, Record<string, number>> = {
  1: { KOR: 168, MATH: 128, ENG: 0, SOC: 0, SCI: 0, PE: 80, MUS: 64, ART: 64, ETH: 34, PRA: 0, CRE: 128 },
  2: { KOR: 168, MATH: 128, ENG: 0, SOC: 0, SCI: 0, PE: 80, MUS: 64, ART: 64, ETH: 34, PRA: 0, CRE: 128 },
  3: { KOR: 204, MATH: 136, ENG: 68, SOC: 102, SCI: 102, PE: 102, MUS: 68, ART: 68, ETH: 34, PRA: 0, CRE: 136 },
  4: { KOR: 204, MATH: 136, ENG: 68, SOC: 102, SCI: 102, PE: 102, MUS: 68, ART: 68, ETH: 34, PRA: 0, CRE: 136 },
  5: { KOR: 170, MATH: 136, ENG: 68, SOC: 136, SCI: 136, PE: 102, MUS: 68, ART: 68, ETH: 34, PRA: 68, CRE: 136 },
  6: { KOR: 170, MATH: 136, ENG: 68, SOC: 136, SCI: 136, PE: 102, MUS: 68, ART: 68, ETH: 34, PRA: 68, CRE: 136 },
}

export default function HoursPage() {
  const params = useParams()
  const yearId = params.id as string

  const [subjects, setSubjects] = useState<(Subject & { hourKey: string })[]>([])
  const [hours, setHours] = useState<Record<string, Record<string, number>>>({})
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [msg, setMsg] = useState('')

  useEffect(() => {
    fetch('/api/curriculum/subjects')
      .then(r => r.json())
      .then(d => {
        const arr: Subject[] = Array.isArray(d) ? d : []
        const keyMap: Record<string, string> = {
          '국어': 'KOR', '수학': 'MATH', '영어': 'ENG', '사회': 'SOC', '과학': 'SCI',
          '체육': 'PE', '음악': 'MUS', '미술': 'ART', '도덕': 'ETH', '실과': 'PRA',
          '창의적체험활동': 'CRE',
        }
        setSubjects(arr.map(s => ({ ...s, hourKey: keyMap[s.name] || s.code })))
      })
      .catch(() => {})
  }, [])

  useEffect(() => {
    fetch(`/api/curriculum/hours?yearId=${yearId}`)
      .then(r => r.json())
      .then(d => {
        const arr: CurriculumHour[] = Array.isArray(d) ? d : []
        const h: Record<string, Record<string, number>> = {}
        GRADES.forEach(g => {
          h[String(g)] = {}
          for (const sub of subjects) h[String(g)][sub.id] = DEFAULT_HOURS[g]?.[sub.hourKey] || 0
        })
        for (const e of arr) {
          if (!h[String(e.grade)]) h[String(e.grade)] = {}
          h[String(e.grade)][e.subjectId] = e.totalHours
        }
        setHours(h)
        setLoading(false)
      })
      .catch(() => setLoading(false))
  }, [yearId, subjects])

  function update(grade: number, subjectId: string, val: number) {
    setHours(prev => ({
      ...prev,
      [String(grade)]: { ...prev[String(grade)], [subjectId]: val },
    }))
  }

  async function save() {
    setSaving(true)
    setMsg('')
    const body: any[] = []
    for (const g of GRADES) {
      for (const sub of subjects) {
        const val = hours[String(g)]?.[sub.id]
        if (val !== undefined && val > 0) {
          body.push({
            year_id: yearId,
            grade: g,
            subject_id: sub.id,
            total_hours: val,
            term1_hours: Math.round(val / 2),
            term2_hours: val - Math.round(val / 2),
          })
        }
      }
    }
    const res = await fetch('/api/curriculum/hours', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    })
    if (res.ok) setMsg('✅ 저장 완료')
    else { const e = await res.json(); setMsg('❌ ' + (e.error || '실패')) }
    setSaving(false)
    setTimeout(() => setMsg(''), 2000)
  }

  function gradeTotal(grade: number) {
    let sum = 0
    for (const sub of subjects) sum += hours[String(grade)]?.[sub.id] || 0
    return sum
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <div>
          <Link href="/admin/curriculum" className="text-sm text-warm-400 hover:underline">← 교육과정</Link>
          <h1 className="text-xl font-bold text-warm-800">⏱️ 시수편제</h1>
        </div>
        <button onClick={save} disabled={saving} className="btn btn-primary">
          {saving ? '저장중...' : '💾 저장'}
        </button>
      </div>

      {msg && <div className="text-sm mb-3" style={{color: msg.startsWith('❌') ? '#dc2626' : '#16a34a'}}>{msg}</div>}

      {loading ? (
        <div className="card text-center py-8 text-warm-400">로딩중...</div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full border-collapse text-sm">
            <thead>
              <tr>
                <th className="border p-2 bg-warm-50 text-warm-500 text-left">교과</th>
                {GRADES.map(g => <th key={g} className="border p-2 bg-warm-50 text-warm-600 w-20">{g}학년</th>)}
              </tr>
            </thead>
            <tbody>
              {subjects.filter(s => s.hourKey !== 'CRE').map(sub => (
                <tr key={sub.id}>
                  <td className="border p-2 text-warm-700 font-medium">{sub.name}</td>
                  {GRADES.map(g => {
                    const def = DEFAULT_HOURS[g]?.[sub.hourKey] || 0
                    return (
                      <td key={g} className="border p-1 text-center">
                        <input type="number" value={hours[String(g)]?.[sub.id] ?? def}
                          onChange={e => update(g, sub.id, +e.target.value)}
                          className="w-full text-center border-0 bg-transparent text-sm py-1 focus:ring-1 focus:ring-blue-300 rounded" />
                      </td>
                    )
                  })}
                </tr>
              ))}
              {/* 창체 */}
              {subjects.filter(s => s.hourKey === 'CRE').map(sub => (
                <tr key={sub.id}>
                  <td className="border p-2 text-warm-700 font-medium">{sub.name}</td>
                  {GRADES.map(g => {
                    const def = DEFAULT_HOURS[g]?.[sub.hourKey] || 0
                    return (
                      <td key={g} className="border p-1 text-center">
                        <input type="number" value={hours[String(g)]?.[sub.id] ?? def}
                          onChange={e => update(g, sub.id, +e.target.value)}
                          className="w-full text-center border-0 bg-transparent text-sm py-1 focus:ring-1 focus:ring-blue-300 rounded" />
                      </td>
                    )
                  })}
                </tr>
              ))}
              {/* 합계 */}
              <tr className="bg-warm-50">
                <td className="border p-2 text-warm-600 font-bold">연간 총 수업일수</td>
                {GRADES.map(g => (
                  <td key={g} className="border p-2 text-center text-warm-700 font-bold">
                    {gradeTotal(g)}일
                  </td>
                ))}
              </tr>
            </tbody>
          </table>
        </div>
      )}

      <div className="mt-4 text-xs text-warm-400 bg-warm-50 rounded p-3">
        <p className="font-medium text-warm-500 mb-1">📌 안내</p>
        <p>• 시수는 2022 개정 교육과정 기준 자동 입력됩니다.</p>
        <p>• 1일 1교시 기준 수업일수로 표시됩니다.</p>
        <p>• 학기별 시수는 총시수의 1/2(1학기) + 나머지(2학기)로 자동 분배됩니다.</p>
      </div>
    </div>
  )
}
