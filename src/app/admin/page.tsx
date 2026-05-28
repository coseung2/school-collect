'use client'

import { useState, useEffect } from 'react'
import Link from 'next/link'
import { statusBadgeClass, statusLabel, targetLabel } from '@/lib/types'
import type { CollectRun } from '@/lib/types'

export default function AdminPage() {
  const [runs, setRuns] = useState<CollectRun[]>([])
  const [loading, setLoading] = useState(true)

  async function loadRuns() {
    setLoading(true)
    const res = await fetch('/api/runs')
    const data = await res.json()
    setRuns(data)
    setLoading(false)
  }

  useEffect(() => { loadRuns() }, [])

  async function changeStatus(id: string, status: string) {
    await fetch('/api/runs', {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ id, status }),
    })
    loadRuns()
  }

  const openRuns = runs.filter(r => r.status === 'open')
  const closedRuns = runs.filter(r => r.status !== 'open')

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-bold text-warm-800">수합 관리</h1>
        <div className="flex gap-2">
          <Link href="/admin/new" className="btn btn-primary">
            + 새 수합
          </Link>
        </div>
      </div>

      {loading ? (
        <div className="card text-center py-8 text-warm-400">로딩중...</div>
      ) : runs.length === 0 ? (
        <div className="card text-center py-12 text-warm-400">
          <div className="text-3xl mb-2">📋</div>
          <div className="text-sm">아직 등록된 수합이 없습니다</div>
          <Link href="/admin/new" className="btn btn-primary mt-4 inline-block">
            첫 수합 만들기
          </Link>
        </div>
      ) : (
        <>
          {/* 진행중 */}
          {openRuns.map(run => (
            <div key={run.id} className="card mb-3">
              <div className="flex items-start justify-between">
                <div className="flex-1">
                  <div className="font-medium text-warm-800">
                    {run.template?.title || run.description || '(제목없음)'}
                  </div>
                  <div className="flex items-center gap-3 mt-1 text-xs text-warm-500">
                    <span>🎯 {targetLabel(run)}</span>
                    <span>📅 ~{new Date(run.deadline).toLocaleDateString('ko-KR')}</span>
                    <span>📊 {run.submissionCount ?? 0}건</span>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <Link href={`/collect/${run.id}`} className="btn btn-secondary text-xs px-2.5 py-1">
                    입력
                  </Link>
                  <Link href={`/collect/${run.id}/results`} className="btn btn-secondary text-xs px-2.5 py-1">
                    결과
                  </Link>
                  <button onClick={() => changeStatus(run.id, 'closed')} className="btn btn-ghost text-xs px-2.5 py-1">
                    마감
                  </button>
                </div>
              </div>
            </div>
          ))}

          {/* 마감된 수합 */}
          {closedRuns.length > 0 && (
            <section className="mt-8">
              <h2 className="text-sm font-semibold text-warm-500 mb-3">마감 / 보관</h2>
              {closedRuns.map(run => (
                <div key={run.id} className="card mb-2 py-3">
                  <div className="flex items-center justify-between">
                    <div>
                      <span className="text-sm text-warm-600">{run.template?.title}</span>
                      <span className={`ml-2 badge ${statusBadgeClass(run.status)} text-xs`}>
                        {statusLabel(run.status)}
                      </span>
                    </div>
                    <div className="flex items-center gap-2">
                      <Link href={`/collect/${run.id}/results`} className="btn btn-ghost text-xs px-2 py-1">결과</Link>
                      <button onClick={() => changeStatus(run.id, 'open')} className="btn btn-ghost text-xs px-2 py-1">
                        재개
                      </button>
                    </div>
                  </div>
                </div>
              ))}
            </section>
          )}
        </>
      )}
    </div>
  )
}
