import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'

// ─── 조회 (planId 기준) ───
export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const planId = searchParams.get('planId')
  const status = searchParams.get('status')
  const sb = getServiceSupabase()

  if (!planId) {
    return NextResponse.json({ error: 'planId required' }, { status: 400 })
  }

  let query = sb.from('monthly_plan_entries')
    .select('*')
    .eq('plan_id', planId)
    .order('plan_date', { ascending: true })

  if (status) query = query.eq('status', status)

  const { data, error } = await query
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data)
}

// ─── 생성 / 일괄 저장 ───
export async function POST(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()

  // batch: entries array
  if (body.entries && Array.isArray(body.entries)) {
    const rows = body.entries.map((e: any) => ({
      plan_id: body.planId,
      plan_date: e.planDate,
      content: e.content,
      target: e.target || '',
      location: e.location || '',
      person_in_charge: e.personInCharge || '',
      notes: e.notes || '',
      submitter: e.submitter || body.submitter || '',
      status: 'draft',
    }))

    const { data, error } = await sb.from('monthly_plan_entries').insert(rows).select()
    if (error) return NextResponse.json({ error: error.message }, { status: 500 })
    return NextResponse.json(data, { status: 201 })
  }

  // single entry
  const { planId, planDate, content, target, location, personInCharge, notes, submitter } = body
  const { data, error } = await sb.from('monthly_plan_entries').insert({
    plan_id: planId,
    plan_date: planDate,
    content,
    target: target || '',
    location: location || '',
    person_in_charge: personInCharge || '',
    notes: notes || '',
    submitter: submitter || '',
  }).select().single()

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data, { status: 201 })
}

// ─── 상태 변경 / 수정 ───
export async function PATCH(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()
  const { id, status, planDate, content, target, location, personInCharge, notes } = body

  if (id && status) {
    const { data, error } = await sb.from('monthly_plan_entries')
      .update({ status })
      .eq('id', id)
      .select()
      .single()
    if (error) return NextResponse.json({ error: error.message }, { status: 500 })
    return NextResponse.json(data)
  }

  if (id) {
    const updates: any = {}
    if (planDate !== undefined) updates.plan_date = planDate
    if (content !== undefined) updates.content = content
    if (target !== undefined) updates.target = target
    if (location !== undefined) updates.location = location
    if (personInCharge !== undefined) updates.person_in_charge = personInCharge
    if (notes !== undefined) updates.notes = notes
    updates.updated_at = new Date().toISOString()

    const { data, error } = await sb.from('monthly_plan_entries')
      .update(updates)
      .eq('id', id)
      .select()
      .single()
    if (error) return NextResponse.json({ error: error.message }, { status: 500 })
    return NextResponse.json(data)
  }

  return NextResponse.json({ error: 'id required' }, { status: 400 })
}

// ─── 삭제 ───
export async function DELETE(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const id = searchParams.get('id')
  const sb = getServiceSupabase()

  if (!id) {
    return NextResponse.json({ error: 'id required' }, { status: 400 })
  }

  const { error } = await sb.from('monthly_plan_entries').delete().eq('id', id)
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json({ success: true })
}
