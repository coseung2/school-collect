'use client'

import { useEffect, useState } from 'react'
import type { PrivacyMaintenanceEntry, PrivacyFileStandard, PrivacyDepartment } from '@/lib/types'

export default function PrivacySubmitPage() {
  const params = typeof window !== 'undefined'
    ? Object.fromEntries(new URLSearchParams(window.location.search))
    : {}
  const deptName = params.dept || ''
  const cycleId = params.cycleId || ''

  const [dept, setDept] = useState<PrivacyDepartment | null>(null)
  const [entries, setEntries] = useState<PrivacyMaintenanceEntry[]>([])
  const [saving, setSaving] = useState(false)
  const [loaded, setLoaded] = useState(false)

  useEffect(() => {
    if (!deptName || !cycleId) return

    // 부서 정보 조회
    fetch(`/api/privacy/departments?cycleId=${cycleId}`)
      .then(r => r.json())
      .then((depts: PrivacyDepartment[]) => {
        const found = depts.find((d: PrivacyDepartment) => d.name === deptName)
        if (found) {
          setDept(found)
          setEntries(found.entries || [])
        }
        setLoaded(true)
      })
  }, [deptName, cycleId])

  const updateEntry = (id: string, field: string, value: unknown) => {
    setEntries(prev =>
      prev.map(e => e.id === id ? { ...e, [field]: value } : e)
    )
  }

  const saveEntry = async (entry: PrivacyMaintenanceEntry) => {
    setSaving(true)
    await fetch('/api/privacy/entries', {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        id: entry.id,
        dataSubjectCount: entry.dataSubjectCount,
        hasAnomaly: entry.hasAnomaly || false,
        anomalyDescription: entry.anomalyDescription || null,
        managerName: entry.managerName || null,
        notes: entry.notes || null,
      }),
    })
    setSaving(false)
  }

  const submitAll = async () => {
    if (!confirm('제출하면 더 이상 수정할 수 없습니다. 제출하시겠습니까?')) return
    setSaving(true)
    for (const entry of entries) {
      await fetch('/api/privacy/entries', {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          id: entry.id,
          dataSubjectCount: entry.dataSubjectCount,
          hasAnomaly: entry.hasAnomaly || false,
          anomalyDescription: entry.anomalyDescription || null,
          managerName: entry.managerName || null,
          notes: entry.notes || null,
          submit: true,
        }),
      })
    }
    setSaving(false)
    alert('✅ 제출 완료되었습니다!')
    window.location.reload()
  }

  if (!deptName) {
    return (
      <div className="py-12 text-center">
        <h1 className="text-xl font-bold mb-4">✏️ 개인정보파일 정비 입력</h1>
        <p className="text-gray-500 mb-4">부서를 선택해주세요:</p>
        <div className="flex gap-3 justify-center flex-wrap">
          {['교무부', '행정실', '보건실', '정보부', '사서'].map(name => (
            <a
              key={name}
              href={`/privacy/submit?dept=${encodeURIComponent(name)}`}
              className="bg-white border border-gray-300 px-6 py-3 text-sm hover:bg-gray-50"
            >
              {name}
            </a>
          ))}
        </div>
      </div>
    )
  }

  if (!loaded) {
    return <div className="text-center py-12 text-gray-500">로딩 중...</div>
  }

  const allSubmitted = entries.every(e => e.isSubmitted)
  const progress = entries.length > 0
    ? Math.round((entries.filter(e => e.dataSubjectCount !== null).length / entries.length) * 100)
    : 0

  return (
    <div>
      <div className="mb-6">
        <a href="/privacy" className="text-sm text-blue-600 hover:underline">← 대시보드로</a>
        <h1 className="text-2xl font-bold mt-2">✏️ {deptName} — 개인정보파일 정비</h1>
        <p className="text-sm text-gray-500 mt-1">
          각 파일의 정보주체 수를 입력하고 제출해주세요.
          {dept?.currentManager && (
            <span className="ml-2">담당자: {dept.currentManager.name}</span>
          )}
        </p>
      </div>

      {/* 진행률 */}
      <div className="w-full bg-gray-100 h-3 mb-6">
        <div className={`h-3 ${allSubmitted ? 'bg-green-500' : 'bg-blue-500'}`} style={{ width: `${progress}%` }} />
      </div>

      {/* 파일별 입력 */}
      <div className="space-y-3">
        {entries.map(entry => {
          const standard = entry.standard
          return (
            <div key={entry.id} className={`bg-white border ${entry.isSubmitted ? 'border-green-200 bg-green-50/30' : 'border-gray-200'}`}>
              <div className="p-4">
                <div className="flex items-start justify-between mb-3">
                  <div>
                    <div className="font-medium">{standard?.fileName || '-'}</div>
                    <div className="text-sm text-gray-500 mt-0.5">
                      {standard?.businessArea} · 보유기간: {standard?.retentionPeriod}
                    </div>
                  </div>
                  {entry.isSubmitted && (
                    <span className="text-xs bg-green-50 text-green-600 px-2 py-0.5 border border-green-200">✅ 제출완료</span>
                  )}
                </div>

                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <label className="text-xs text-gray-500 block mb-1">정보주체 수</label>
                    <input
                      type="number"
                      className="w-full border border-gray-300 px-3 py-1.5 text-sm"
                      value={entry.dataSubjectCount ?? ''}
                      onChange={e => updateEntry(entry.id, 'dataSubjectCount', e.target.value ? parseInt(e.target.value) : null)}
                      disabled={entry.isSubmitted}
                      placeholder="숫자 입력"
                    />
                    {standard?.guide && (
                      <details className="mt-1">
                        <summary className="text-xs text-blue-500 cursor-pointer">산정방법</summary>
                        <p className="text-xs text-gray-500 mt-1">{standard.guide}</p>
                      </details>
                    )}
                  </div>
                  <div>
                    <label className="text-xs text-gray-500 block mb-1">이상유무</label>
                    <select
                      className="w-full border border-gray-300 px-3 py-1.5 text-sm"
                      value={entry.hasAnomaly ? 'yes' : 'no'}
                      onChange={e => updateEntry(entry.id, 'hasAnomaly', e.target.value === 'yes')}
                      disabled={entry.isSubmitted}
                    >
                      <option value="no">이상 없음</option>
                      <option value="yes">이상 있음</option>
                    </select>
                  </div>
                </div>

                {entry.hasAnomaly && (
                  <div className="mt-2">
                    <label className="text-xs text-gray-500 block mb-1">이상 내용</label>
                    <input
                      type="text"
                      className="w-full border border-gray-300 px-3 py-1.5 text-sm"
                      value={entry.anomalyDescription || ''}
                      onChange={e => updateEntry(entry.id, 'anomalyDescription', e.target.value)}
                      disabled={entry.isSubmitted}
                      placeholder="이상 내용 입력"
                    />
                  </div>
                )}

                <div className="mt-2">
                  <label className="text-xs text-gray-500 block mb-1">입력자 (선택)</label>
                  <input
                    type="text"
                    className="w-full border border-gray-300 px-3 py-1.5 text-sm"
                    value={entry.managerName || ''}
                    onChange={e => updateEntry(entry.id, 'managerName', e.target.value)}
                    disabled={entry.isSubmitted}
                    placeholder="담당자 성명"
                  />
                </div>
              </div>
            </div>
          )
        })}
      </div>

      {/* 액션 버튼 */}
      {entries.length > 0 && !allSubmitted && (
        <div className="mt-6 flex gap-3">
          <button
            className="bg-blue-600 text-white px-6 py-2 text-sm hover:bg-blue-700 disabled:opacity-50"
            onClick={submitAll}
            disabled={saving}
          >
            {saving ? '저장 중...' : '✅ 일괄 제출'}
          </button>
          <button
            className="bg-white border border-gray-300 px-4 py-2 text-sm hover:bg-gray-50 disabled:opacity-50"
            onClick={async () => {
              for (const entry of entries) {
                if (!entry.isSubmitted) await saveEntry(entry)
              }
              alert('저장 완료! (아직 제출되지 않음)')
            }}
            disabled={saving}
          >
            💾 임시 저장
          </button>
        </div>
      )}
    </div>
  )
}
