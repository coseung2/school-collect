'use client'

import { useState, useEffect } from 'react'
import { useParams } from 'next/navigation'
import Link from 'next/link'
import { statusBadgeClass, statusLabel, targetLabel } from '@/lib/types'
import type { CollectRun, FormField, CollectSubmission } from '@/lib/types'

interface RunDetail extends Omit<CollectRun, 'template'> {
  template: {
    id: string
    title: string
    description: string | null
    year: number
    created_at: string
    updated_at: string
    fields: FormField[]
  }
}

export default function ResultsPage() {
  const { id } = useParams<{ id: string }>()
  const [run, setRun] = useState<RunDetail | null>(null)
  const [submissions, setSubmissions] = useState<CollectSubmission[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    Promise.all([
      fetch('/api/runs?status=open').then(r => r.json()),
      fetch('/api/runs?status=closed').then(r => r.json()),
      fetch('/api/templates').then(r => r.json()),
      fetch(`/api/submissions?runId=${id}`).then(r => r.json()),
    ]).then(([openRuns, closedRuns, templates, subs]) => {
      const allRuns = [...openRuns, ...closedRuns]
      const runData = allRuns.find((r: any) => r.id === id)
      if (runData) {
        const template = templates.find((t: any) => t.id === runData.templateId || t.id === runData.template?.id)
        setRun({ ...runData, template: template || runData.template })
      }
      setSubmissions(subs || [])
      setLoading(false)
    })
  }, [id])

  if (loading) return <div className="card text-center py-8 text-warm-400">로딩중...</div>
  if (!run) return <div className="card text-center py-8 text-warm-400">수합을 찾을 수 없습니다</div>

  const fields = (run.template?.fields || []).sort((a, b) => a.order - b.order)
  const totalNeeded = run.targetType === 'all' ? 12 : // 가정
    run.targetValue ? JSON.parse(run.targetValue).length * 6 : 6

  return (
    <div>
      {/* 헤더 */}
      <div className="flex items-center justify-between mb-4">
        <div>
          <h1 className="text-xl font-bold text-warm-800">
            {run.template?.title || '수합'} — 취합 현황
          </h1>
          <div className="flex items-center gap-3 mt-1 text-xs text-warm-500">
            <span>🎯 {targetLabel(run)}</span>
            <span>📅 ~{new Date(run.deadline).toLocaleDateString('ko-KR')}</span>
            <span className={`badge ${statusBadgeClass(run.status)}`}>{statusLabel(run.status)}</span>
          </div>
        </div>
        <div className="flex gap-2">
          <Link href={`/collect/${id}`} className="btn btn-secondary text-sm">
            입력 폼
          </Link>
        </div>
      </div>

      {/* 진행률 */}
      <div className="card mb-6">
        <div className="flex items-center justify-between mb-2">
          <span className="text-sm font-medium text-warm-700">제출 현황</span>
          <span className="text-sm text-warm-500">{submissions.length} / {totalNeeded}건</span>
        </div>
        <div className="w-full bg-warm-100 rounded-full h-2">
          <div className="bg-emerald-500 h-2 rounded-full transition-all"
            style={{ width: `${Math.min(100, (submissions.length / Math.max(1, totalNeeded)) * 100)}%` }} />
        </div>
      </div>

      {/* 테이블 */}
      <div className="card !p-0 overflow-hidden">
        <div className="table-wrap">
          <table className="data">
            <thead>
              <tr>
                <th className="sticky left-0 bg-white z-10">학년-반</th>
                <th>제출자</th>
                <th>제출일</th>
                {fields.map(f => (
                  <th key={f.id}>{f.label}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {submissions.length === 0 ? (
                <tr>
                  <td colSpan={3 + fields.length} className="text-center py-8 text-warm-400">
                    아직 제출된 데이터가 없습니다
                  </td>
                </tr>
              ) : (
                submissions.map(sub => (
                  <tr key={sub.id}>
                    <td className="font-medium text-warm-700">{sub.grade}-{sub.classNum}</td>
                    <td className="text-warm-500">{sub.submitter || '-'}</td>
                    <td className="text-warm-500 text-xs">
                      {new Date(sub.submittedAt).toLocaleDateString('ko-KR')}
                    </td>
                    {fields.map(f => {
                      const answer = sub.answers?.find(a => a.fieldId === f.id)
                      return (
                        <td key={f.id} className="text-warm-700 max-w-[200px] truncate">
                          {answer?.value || '-'}
                        </td>
                      )
                    })}
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* 엑셀 다운로드 (CSV) */}
      {submissions.length > 0 && (
        <div className="mt-4 flex gap-2">
          <button
            onClick={() => {
              const headers = ['학년-반', '제출자', '제출일', ...fields.map(f => f.label)]
              const rows = submissions.map(sub => [
                `${sub.grade}-${sub.classNum}`,
                sub.submitter || '',
                new Date(sub.submittedAt).toLocaleDateString('ko-KR'),
                ...fields.map(f => {
                  const answer = sub.answers?.find(a => a.fieldId === f.id)
                  return answer?.value || ''
                })
              ])
              const csv = [headers.join(','), ...rows.map(r => r.map(c => `"${c}"`).join(','))].join('\n')
              const blob = new Blob(['\uFEFF' + csv], { type: 'text/csv;charset=utf-8;' })
              const url = URL.createObjectURL(blob)
              const a = document.createElement('a')
              a.href = url
              a.download = `${run.template?.title || '수합'}_취합.csv`
              a.click()
              URL.revokeObjectURL(url)
            }}
            className="btn btn-secondary text-sm">
            📥 CSV 다운로드
          </button>
        </div>
      )}
    </div>
  )
}
