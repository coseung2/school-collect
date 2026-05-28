'use client'

import { useEffect, useState } from 'react'
import type { DashboardSummary, PrivacyMaintenanceCycle } from '@/lib/types'
import { useAuth } from '@/lib/useAuth'

export default function PrivacyDashboard() {
  const session = useAuth()
  const [data, setData] = useState<DashboardSummary | null>(null)
  const [cycles, setCycles] = useState<PrivacyMaintenanceCycle[]>([])
  const [selectedCycleId, setSelectedCycleId] = useState('')
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    fetch('/api/privacy/cycles')
      .then(r => r.json())
      .then(setCycles)
  }, [])

  useEffect(() => {
    if (!selectedCycleId && cycles.length > 0) {
      setSelectedCycleId(cycles[0].id)
    }
  }, [cycles, selectedCycleId])

  useEffect(() => {
    if (!selectedCycleId) return
    setLoading(true)
    fetch(`/api/privacy/dashboard?cycleId=${selectedCycleId}`)
      .then(r => r.json())
      .then(d => { setData(d); setLoading(false) })
      .catch(() => setLoading(false))
  }, [selectedCycleId])

  const cycle = cycles.find(c => c.id === selectedCycleId)
  const progress = data ? Math.round((data.submittedEntries / (data.totalEntries || 1)) * 100) : 0

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold">📋 개인정보파일 정비 대시보드</h1>
        <select
          className="border border-gray-300 px-3 py-1.5 text-sm"
          value={selectedCycleId}
          onChange={e => setSelectedCycleId(e.target.value)}
        >
          {cycles.map(c => (
            <option key={c.id} value={c.id}>
              {c.year}년 정비 ({c.status === 'draft' ? '초안' : c.status === 'active' ? '진행중' : '완료'})
            </option>
          ))}
        </select>
      </div>

      {loading ? (
        <div className="text-center py-12 text-gray-500">로딩 중...</div>
      ) : !data ? (
        <div className="text-center py-12 text-gray-500">
          정비 사이클이 없습니다. 관리자 페이지에서 생성해주세요.
        </div>
      ) : (
        <>
          {/* 상단 요약 */}
          <div className="grid grid-cols-4 gap-4 mb-6">
            <div className="bg-white border border-gray-200 p-4">
              <div className="text-sm text-gray-500">전체 진행률</div>
              <div className="text-2xl font-bold mt-1">{progress}%</div>
              <div className="w-full bg-gray-100 h-2 mt-2">
                <div className="bg-blue-500 h-2" style={{ width: `${progress}%` }} />
              </div>
            </div>
            <div className="bg-white border border-gray-200 p-4">
              <div className="text-sm text-gray-500">제출 완료 부서</div>
              <div className="text-2xl font-bold mt-1 text-green-600">
                {data.completedDepartments}/{data.totalDepartments}
              </div>
            </div>
            <div className="bg-white border border-gray-200 p-4">
              <div className="text-sm text-gray-500">제출/전체 건수</div>
              <div className="text-2xl font-bold mt-1">
                {data.submittedEntries}/{data.totalEntries}
              </div>
            </div>
            <div className="bg-white border border-gray-200 p-4">
              <div className="text-sm text-gray-500">정비 기간</div>
              <div className="text-lg font-bold mt-1">
                {cycle?.startDate ? new Date(cycle.startDate).toLocaleDateString('ko-KR') : '-'}
                {' ~ '}
                {cycle?.endDate ? new Date(cycle.endDate).toLocaleDateString('ko-KR') : '-'}
              </div>
            </div>
          </div>

          {/* 부서별 현황 */}
          <div className="bg-white border border-gray-200">
            <div className="px-4 py-3 border-b border-gray-200 font-bold">부서별 진행 현황</div>
            {data.departmentProgress.map((dp) => (
              <div key={dp.department.id} className="border-b border-gray-100 last:border-b-0">
                <div className="px-4 py-3 flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <span className="font-medium">{dp.department.name}</span>
                    {dp.department.currentManager && (
                      <span className="text-sm text-gray-500">
                        ({dp.department.currentManager.name})
                      </span>
                    )}
                    {dp.pendingFiles === 0 ? (
                      <span className="text-xs bg-green-50 text-green-600 px-2 py-0.5 border border-green-200">✅ 완료</span>
                    ) : dp.submittedFiles > 0 ? (
                      <span className="text-xs bg-yellow-50 text-yellow-600 px-2 py-0.5 border border-yellow-200">🔄 진행중</span>
                    ) : (
                      <span className="text-xs bg-red-50 text-red-600 px-2 py-0.5 border border-red-200">⏳ 미제출</span>
                    )}
                  </div>
                  <div className="flex items-center gap-4">
                    <span className="text-sm text-gray-600">
                      {dp.submittedFiles}/{dp.totalFiles}건
                    </span>
                    <a
                      href={`/privacy/submit?dept=${encodeURIComponent(dp.department.name)}&cycleId=${selectedCycleId}`}
                      className="text-sm text-blue-600 hover:underline"
                    >
                      입력하기 →
                    </a>
                  </div>
                </div>
                {/* 파일 상세 (접이식) */}
                {dp.entries.length > 0 && (
                  <details className="px-4 pb-2">
                    <summary className="text-xs text-gray-400 cursor-pointer">파일 상세 보기</summary>
                    <div className="mt-2 space-y-1">
                      {dp.entries.map(entry => (
                        <div key={entry.id} className="flex items-center justify-between text-sm py-1 px-2 bg-gray-50">
                          <span>{entry.standard?.fileName || '-'}</span>
                          <span className="text-gray-500">
                            {entry.isSubmitted
                              ? `✅ ${entry.dataSubjectCount ?? '?'}명`
                              : '⬜ 미입력'}
                          </span>
                        </div>
                      ))}
                    </div>
                  </details>
                )}
              </div>
            ))}
          </div>

          {/* 액션 버튼 */}
          {cycle?.status !== 'completed' && (
            <div className="mt-6 flex gap-3">
              <a
                href={`/api/privacy/export?cycleId=${selectedCycleId}&format=csv`}
                className="bg-white border border-gray-300 px-4 py-2 text-sm hover:bg-gray-50"
              >
                📥 엑셀(CSV) 추출
              </a>
              {cycle?.status === 'active' && data.pendingEntries === 0 && (
                <button
                  className="bg-blue-600 text-white px-4 py-2 text-sm hover:bg-blue-700"
                  onClick={async () => {
                    if (!confirm('모든 부서가 제출 완료되었습니다. 정비를 마무리할까요?')) return
                    await fetch('/api/privacy/export', {
                      method: 'POST',
                      headers: { 'Content-Type': 'application/json' },
                      body: JSON.stringify({ cycleId: selectedCycleId }),
                    })
                    window.location.reload()
                  }}
                >
                  ✅ 정비 완료 처리
                </button>
              )}
            </div>
          )}
        </>
      )}
    </div>
  )
}
