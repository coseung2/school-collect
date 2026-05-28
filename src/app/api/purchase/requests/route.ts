import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'

// ─── 특정 물품의 신청 목록 ───
export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const itemId = searchParams.get('itemId')
  const sb = getServiceSupabase()

  let query = sb.from('purchase_requests').select(`
    *,
    item:purchase_items(*)
  `)

  if (itemId) {
    query = query.eq('item_id', itemId)
  }

  const { data, error } = await query
    .order('grade', { ascending: true })
    .order('class_num', { ascending: true })

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })

  const result = (data || []).map((r: any) => ({
    id: r.id,
    itemId: r.item_id,
    grade: r.grade,
    classNum: r.class_num,
    quantity: r.quantity,
    submitter: r.submitter || '',
    notes: r.notes || '',
    createdAt: r.created_at,
    item: r.item ? {
      id: r.item.id,
      name: r.item.name,
      unitPrice: r.item.unit_price,
      unit: r.item.unit,
    } : undefined,
  }))

  return NextResponse.json(result)
}

// ─── 신청하기 (Upsert: 학년+반 1회만) ───
export async function POST(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()
  const { itemId, grade, classNum, quantity, submitter, notes } = body

  if (!itemId || !grade) {
    return NextResponse.json({ error: '물품과 학년은 필수입니다' }, { status: 400 })
  }

  // Upsert: 이미 신청했으면 수정, 없으면 추가
  const { data: existing } = await sb.from('purchase_requests')
    .select('id')
    .eq('item_id', itemId)
    .eq('grade', grade)
    .eq('class_num', classNum)
    .maybeSingle()

  if (existing) {
    const { error } = await sb.from('purchase_requests')
      .update({
        quantity: quantity || 1,
        submitter: submitter || '',
        notes: notes || '',
      })
      .eq('id', existing.id)

    if (error) return NextResponse.json({ error: error.message }, { status: 500 })
    return NextResponse.json({ id: existing.id, updated: true })
  }

  const { data, error } = await sb.from('purchase_requests').insert({
    item_id: itemId,
    grade,
    class_num: classNum,
    quantity: quantity || 1,
    submitter: submitter || '',
    notes: notes || '',
  }).select().single()

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json({ id: data.id, updated: false }, { status: 201 })
}

// ─── 신청 삭제 ───
export async function DELETE(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const id = searchParams.get('id')
  if (!id) return NextResponse.json({ error: 'id required' }, { status: 400 })

  const sb = getServiceSupabase()
  const { error } = await sb.from('purchase_requests').delete().eq('id', id)
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })

  return NextResponse.json({ deleted: true })
}