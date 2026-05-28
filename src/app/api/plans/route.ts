import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'

// ─── 목록/단일 조회 ───
export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const id = searchParams.get('id')
  const year = searchParams.get('year')
  const month = searchParams.get('month')
  const status = searchParams.get('status')
  const sb = getServiceSupabase()

  if (id) {
    const { data, error } = await sb.from('monthly_plans')
      .select(`*, entries:monthly_plan_entries(*)`)
      .eq('id', id)
      .single()
    if (error) return NextResponse.json({ error: error.message }, { status: 500 })
    return NextResponse.json(data)
  }

  let query = sb.from('monthly_plans')
    .select('*')
    .order('year', { ascending: false })
    .order('month', { ascending: false })

  if (year) query = query.eq('year', parseInt(year))
  if (month) query = query.eq('month', parseInt(month))
  if (status) query = query.eq('status', status)

  const { data, error } = await query
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })

  // add entry counts
  const withCounts = await Promise.all((data || []).map(async (plan) => {
    const { count: total } = await sb.from('monthly_plan_entries')
      .select('*', { count: 'exact', head: true })
      .eq('plan_id', plan.id)
    const { count: confirmed } = await sb.from('monthly_plan_entries')
      .select('*', { count: 'exact', head: true })
      .eq('plan_id', plan.id).eq('status', 'confirmed')
    const { count: submitted } = await sb.from('monthly_plan_entries')
      .select('*', { count: 'exact', head: true })
      .eq('plan_id', plan.id).eq('status', 'submitted')
    return {
      ...plan,
      entryCount: total || 0,
      confirmedCount: confirmed || 0,
      submittedCount: submitted || 0,
    }
  }))

  return NextResponse.json(withCounts)
}

// ─── 생성 ───
export async function POST(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()
  const { year, month, title, description, deadline } = body

  const { data, error } = await sb.from('monthly_plans').insert({
    year: year || new Date().getFullYear(),
    month,
    title,
    description: description || null,
    deadline: deadline || null,
  }).select().single()

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data, { status: 201 })
}

// ─── 상태 변경 ───
export async function PATCH(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()
  const { id, status } = body

  if (!id || !status) {
    return NextResponse.json({ error: 'id and status required' }, { status: 400 })
  }

  const { data, error } = await sb.from('monthly_plans')
    .update({ status })
    .eq('id', id)
    .select()
    .single()

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data)
}
