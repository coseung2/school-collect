'use client'

import { useState, useEffect } from 'react'
import { useParams } from 'next/navigation'
import Link from 'next/link'
import type { CurriculumClass } from '@/lib/types'

const GRADES = [1, 2, 3, 4, 5, 6]
const DEFAULT_CLASSES = 6

export default function ClassesPage() {
  const params = useParams()
  const yearId = params.id as string

  const [classes, setClasses] = useState<CurriculumClass[]>([])
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [msg, setMsg] = useState('')
  const [config, setConfig] = useState<Record<string, number>>(() => {
    const c: Record<string, number> = {}
    GRADES.forEach(g => { c[String(g)] = DEFAULT_CLASSES })
    return c
  })

  useEffect(() => {
    fetch(`/api/curriculum/classes?curriculumYearId=${yearId}`)
      .then(r => r.json())
      .then(d => {
        const arr: CurriculumClass[] = Array.isArray(d) ? d : []
        setClasses(arr)
        // 기존 설정 반영
        const cfg: Record<string, number> = {}
        GRADES.forEach(g => { cfg[String(g)] = 0 })
        for (const c of arr) {
          if (c.curriculumYearId === yearId) {
            cfg[String(c.grade)] = Math.max(cfg[String(c.grade)], c.classNo)
          }
        }
        // 기본값 유지
        GRADES.forEach(g => {
          if (cfg[String(g)] === 0) cfg[String(g)] = DEFAULT_CLASSES
        })
        setConfig(cfg)
      })
      .catch(() => {})
      .finally(() => setLoading(false))
  }, [yearId])

  async function save() {
    setSaving(true)
    setMsg('')
    // 기존 삭제 후 재생성
    await fetch(`/api/curriculum/classes?curriculumYearId=${yearId}`, { method: 'DELETE' })

    const inserts: any[] = []
    for (const g of GRADES) {
      const cnt = config[String(g)]
      if (!cnt || cnt <= 0) continue
      for (let c = 1; c <= cnt; c++) {
        inserts.push({
          curriculum_year_id: yearId,
          grade: g,
          class_no: c,
        })
      }
    }

    const res = await fetch('/api/curriculum/classes', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(inserts),
    })
    if (res.ok) {
      setMsg('✅ 저장 완료')
      const data = await res.json()
      setClasses(Array.isArray(data) ? data : [])
    } else {
      const e = await res.json()
      setMsg('❌ ' + (e.error || '실패'))
    }
    setSaving(false)
    setTimeout(() => setMsg(''), 2000)
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <div>
          <Link href="/admin/curriculum" className="text-sm text-warm-400 hover:underline">← 교육과정</Link>
          <h1 className="text-xl font-bold text-warm-800">🏫 학년/반 설정</h1>
        </div>
        <button onClick={save} disabled={saving} className="btn btn-primary">
          {saving ? '저장중...' : '💾 저장'}
        </button>
      </div>

      {msg && <div className="text-sm mb-3" style={{color: msg.startsWith('❌') ? '#dc2626' : '#16a34a'}}>{msg}</div>}

      {loading ? (
        <div className="card text-center py-8 text-warm-400">로딩중...</div>
      ) : (
        <div className="grid gap-3 sm:grid-cols-2">
          {GRADES.map(g => (
            <div key={g} className="card">
              <div className="text-sm font-bold text-warm-700 mb-2">{g}학년</div>
              <div className="flex items-center gap-2">
                <span className="text-xs text-warm-400">반 수</span>
                <input type="number" value={config[String(g)]} min={0} max={12}
                  onChange={e => setConfig(prev => ({ ...prev, [String(g)]: +e.target.value }))}
                  className="border rounded px-2 py-1 text-sm w-16" />
                <span className="text-xs text-warm-400">개 반</span>
              </div>
              <div className="text-xs text-warm-300 mt-1">
                총 {config[String(g)] * (g <= 2 ? 22 : g <= 4 ? 26 : 28)}명 예상
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
