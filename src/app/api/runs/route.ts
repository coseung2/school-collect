import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'

// ─── 수합 실행 목록 ───
export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const year = searchParams.get('year')
  const status = searchParams.get('status')
  const sb = getServiceSupabase()

  let query = sb.from('collect_runs').select(`
    *,
    template:form_templates(*)
  `).order('created_at', { ascending: false })

  if (year) query = query.eq('year', parseInt(year))
  if (status) query = query.eq('status', status)

  const { data, error } = await query
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })

  // 제출수 추가
  const runsWithCounts = await Promise.all((data || []).map(async (run) => {
    const { count } = await sb.from('collect_submissions')
      .select('*', { count: 'exact', head: true })
      .eq('run_id', run.id)
    return { ...run, submissionCount: count || 0 }
  }))

  return NextResponse.json(runsWithCounts)
}

// ─── 수합 실행 생성 ───
export async function POST(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()

  const { templateId, year, deadline, targetType, targetValue, description } = body

  const { data, error } = await sb.from('collect_runs').insert({
    template_id: templateId,
    year: year || new Date().getFullYear(),
    deadline,
    status: 'open',
    target_type: targetType || 'all',
    target_value: targetValue || null,
    description: description || null,
  }).select(`
    *,
    template:form_templates(*)
  `).single()

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data, { status: 201 })
}

// ─── 수합 실행 상태 변경 ───
export async function PATCH(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()
  const { id, status } = body

  if (!id || !status) {
    return NextResponse.json({ error: 'id and status required' }, { status: 400 })
  }

  const { data, error } = await sb.from('collect_runs')
    .update({ status })
    .eq('id', id)
    .select()
    .single()

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data)
}
