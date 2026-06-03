'use client'

import { useState, useEffect } from 'react'
import { useParams } from 'next/navigation'
import Link from 'next/link'

export default function ValidatePage() {
  const params = useParams()
  const yearId = params.id as string

  const [result, setResult] = useState<any>(null)
  const [loading, setLoading] = useState(true)
  const [running, setRunning] = useState(false)

  function runValidate() {
    setRunning(true)
    fetch(`/api/curriculum/validate?yearId=${yearId}`)
      .then(r => r.json())
      .then(d => setResult(d))
      .catch(() => setResult({ error: '검증 실패' }))
      .finally(() => { setLoading(false); setRunning(false) })
  }

  useEffect(runValidate, [yearId])

  const FIELD_META: Record<string, { label: string; path: string; emoji: string }> = {
    '시수편제': { label: '시수편제', path: `/admin/curriculum/hours/${yearId}`, emoji: '⏱️' },
    '학사일정': { label: '학사일정', path: `/admin/curriculum/calendar/${yearId}`, emoji: '📅' },
    '기초시간표': { label: '기초시간표', path: `/admin/curriculum/timetable/base/${yearId}`, emoji: '📋' },
    '교과 진도표': { label: '교과 진도표', path: `/admin/curriculum/lessons/${yearId}`, emoji: '📖' },
    '범교과주제': { label: '범교과주제', path: `/admin/curriculum/cross-topics/${yearId}`, emoji: '🏷️' },
  }

  return (
    <div>
      <div className="mb-4">
        <Link href="/admin/curriculum" className="text-sm text-warm-400 hover:underline">← 교육과정</Link>
        <h1 className="text-xl font-bold text-warm-800">✅ 교육과정 검증</h1>
      </div>

      {loading ? (
        <div className="card text-center py-8 text-warm-400">검증중...</div>
      ) : !result ? (
        <div className="card text-center py-8 text-warm-400">검증 결과가 없습니다</div>
      ) : (
        <>
          {/* 진행률 */}
          <div className="card mb-4">
            <div className="flex items-center justify-between mb-2">
              <span className="text-sm font-bold text-warm-700">완성도</span>
              <span className={`text-lg font-bold ${
                result.completeness === 100 ? 'text-green-600' :
                result.completeness >= 60 ? 'text-blue-600' :
                'text-red-500'
              }`}>
                {result.completeness}%
              </span>
            </div>
            <div className="w-full bg-warm-100 rounded-full h-2.5">
              <div className={`h-2.5 rounded-full transition-all ${
                result.completeness === 100 ? 'bg-green-500' :
                result.completeness >= 60 ? 'bg-blue-500' :
                'bg-red-400'
              }`} style={{ width: `${result.completeness}%` }} />
            </div>
          </div>

          {/* 항목별 결과 */}
          <div className="space-y-2">
            {Object.entries(FIELD_META).map(([field, meta]) => {
              const item = result.items?.find((i: any) => i.field === field)
              const done = !item
              return (
                <div key={field} className="card flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <span className={`text-lg ${done ? 'opacity-100' : 'opacity-40'}`}>
                      {done ? '✅' : '⏳'}
                    </span>
                    <div>
                      <span className="font-medium text-warm-800">{meta.emoji} {meta.label}</span>
                      {item && (
                        <div className="text-xs text-red-400 mt-0.5">{item.message}</div>
                      )}
                    </div>
                  </div>
                  <Link href={meta.path}
                    className="text-xs text-blue-500 hover:underline whitespace-nowrap">
                    {done ? '수정' : '작성하기 →'}
                  </Link>
                </div>
              )
            })}
          </div>

          {/* 재검증 버튼 */}
          <div className="text-center mt-6">
            <button onClick={runValidate} disabled={running}
              className="btn btn-primary">
              {running ? '검증중...' : '🔄 다시 검증'}
            </button>
          </div>

          {result.completeness === 100 && (
            <div className="card mt-4 border-2 border-green-200 bg-green-50 text-center py-4">
              <div className="text-3xl mb-2">🎉</div>
              <div className="font-bold text-green-700 text-lg">모든 항목이 완료되었습니다!</div>
              <div className="text-sm text-green-600 mt-1">교육과정 출력 및 확정 단계로 진행할 수 있습니다.</div>
            </div>
          )}
        </>
      )}
    </div>
  )
}
