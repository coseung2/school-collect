import Link from 'next/link'
import { getServiceSupabase } from '@/lib/supabase'
import { statusBadgeClass, statusLabel, targetLabel, planStatusLabel } from '@/lib/types'
import type { CollectRun, MonthlyPlan } from '@/lib/types'

async function getRuns(): Promise<CollectRun[]> {
  const sb = getServiceSupabase()
  const currentYear = new Date().getFullYear()

  const { data } = await sb.from('collect_runs').select(`
    *,
    template:form_templates(*)
  `)
    .eq('year', currentYear)
    .order('deadline', { ascending: true })

  if (!data) return []

  return await Promise.all((data).map(async (run: any) => {
    const { count } = await sb.from('collect_submissions')
      .select('*', { count: 'exact', head: true })
      .eq('run_id', run.id)
    return { ...run, submissionCount: count || 0 }
  }))
}

async function getTemplateCount(): Promise<number> {
  const sb = getServiceSupabase()
  const { count } = await sb.from('form_templates')
    .select('*', { count: 'exact', head: true })
  return count || 0
}

async function getMonthlyPlans(): Promise<MonthlyPlan[]> {
  const sb = getServiceSupabase()
  const currentYear = new Date().getFullYear()

  const { data } = await sb.from('monthly_plans')
    .select('*')
    .eq('year', currentYear)
    .order('month', { ascending: false })

  if (!data) return []

  return await Promise.all((data).map(async (plan: any) => {
    const { count: confirmed } = await sb.from('monthly_plan_entries')
      .select('*', { count: 'exact', head: true })
      .eq('plan_id', plan.id).eq('status', 'confirmed')
    return { ...plan, confirmedCount: confirmed || 0 }
  }))
}

export default async function DashboardPage() {
  const [runs, templateCount, monthlyPlans] = await Promise.all([getRuns(), getTemplateCount(), getMonthlyPlans()])

  const openRuns = runs.filter(r => r.status === 'open')
  const closedRuns = runs.filter(r => r.status === 'closed')

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-bold text-warm-800">대시보드</h1>
        <Link href="/admin/new" className="btn btn-primary">
          + 새 수합 등록
        </Link>
      </div>

      {/* 요약 카드 */}
      <div className="grid grid-cols-3 gap-4 mb-8">
        <div className="card text-center">
          <div className="text-2xl font-bold text-warm-700">{templateCount}</div>
          <div className="text-xs text-warm-500 mt-1">템플릿</div>
        </div>
        <div className="card text-center">
          <div className="text-2xl font-bold text-emerald-600">{openRuns.length}</div>
          <div className="text-xs text-warm-500 mt-1">진행중</div>
        </div>
        <div className="card text-center">
          <div className="text-2xl font-bold text-warm-500">{runs.length}</div>
          <div className="text-xs text-warm-500 mt-1">올해 전체 수합</div>
        </div>
      </div>

      {/* 진행중인 수합 */}
      <section className="mb-8">
        <h2 className="text-base font-semibold text-warm-700 mb-3">📌 진행중인 수합</h2>
        {openRuns.length === 0 ? (
          <div className="card text-center py-8 text-warm-400">
            진행중인 수합이 없습니다
          </div>
        ) : (
          <div className="space-y-3">
            {openRuns.map(run => (
              <Link key={run.id} href={`/collect/${run.id}`} className="card block hover:border-warm-300 transition">
                <div className="flex items-start justify-between">
                  <div>
                    <div className="font-medium text-warm-800">
                      {run.template?.title || run.description || '수합'}
                    </div>
                    <div className="flex items-center gap-3 mt-1 text-xs text-warm-500">
                      <span>🎯 {targetLabel(run)}</span>
                      <span>📅 ~{new Date(run.deadline).toLocaleDateString('ko-KR')}</span>
                    </div>
                  </div>
                  <div className="text-right">
                    <span className={`badge ${statusBadgeClass(run.status)}`}>{statusLabel(run.status)}</span>
                    <div className="text-xs text-warm-400 mt-1">
                      {run.submissionCount ?? 0}건 제출
                    </div>
                  </div>
                </div>
              </Link>
            ))}
          </div>
        )}
      </section>

      {/* 월중계획 */}
      {monthlyPlans.length > 0 && (
        <section className="mb-8">
          <h2 className="text-base font-semibold text-gray-700 mb-3">📅 월중계획</h2>
          <div className="space-y-2">
            {monthlyPlans.map(plan => (
              <Link key={plan.id} href={`/plan/${plan.id}`}
                className="block border border-gray-200 bg-white px-4 py-3 hover:bg-gray-50 transition"
                style={{ borderRadius: 0 }}>
                <div className="flex items-center justify-between">
                  <div>
                    <span className="text-sm font-medium text-gray-800">
                      {plan.title || `${plan.year}년 ${plan.month}월 월중계획`}
                    </span>
                    <span className="text-[10px] text-gray-500 ml-2">
                      {plan.month}월
                    </span>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-[10px] text-gray-400">
                      ✅ {plan.confirmedCount ?? 0}건 확정
                    </span>
                    <span className={`text-[10px] px-1.5 py-0.5 border font-medium
                      ${plan.status === 'published' ? 'bg-green-50 text-green-700 border-green-200' :
                        plan.status === 'reviewing' ? 'bg-yellow-50 text-yellow-700 border-yellow-200' :
                        'bg-gray-50 text-gray-600 border-gray-200'}`}
                      style={{ borderRadius: 0 }}>
                       {planStatusLabel(plan.status)}
                    </span>
                  </div>
                </div>
              </Link>
            ))}
          </div>
        </section>
      )}

      {/* 마감/보관 */}
      {closedRuns.length > 0 && (
        <section>
          <h2 className="text-base font-semibold text-warm-700 mb-3">📁 마감된 수합</h2>
          <div className="space-y-2">
            {closedRuns.map(run => (
              <Link key={run.id} href={`/collect/${run.id}/results`} className="card block py-3 hover:border-warm-300 transition">
                <div className="flex items-center justify-between">
                  <div className="text-sm text-warm-700">{run.template?.title}</div>
                  <div className="flex items-center gap-3">
                    <span className="text-xs text-warm-400">{run.submissionCount ?? 0}건</span>
                    <span className={`badge ${statusBadgeClass(run.status)}`}>{statusLabel(run.status)}</span>
                  </div>
                </div>
              </Link>
            ))}
          </div>
        </section>
      )}
    </div>
  )
}
