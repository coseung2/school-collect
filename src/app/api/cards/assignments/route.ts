import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'

export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const cardId = searchParams.get('cardId')
  const active = searchParams.get('active') // 'true' = 미반납만
  const sb = getServiceSupabase()

  let query = sb.from('card_assignments').select(`
    *,
    card:card_expense_cards(*)
  `).order('taken_at', { ascending: false })

  if (cardId) query = query.eq('card_id', cardId)
  if (active === 'true') query = query.is('returned_at', null)

  const { data, error } = await query
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data || [])
}

export async function POST(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()
  const { cardId, teacherName, note } = body

  if (!cardId || !teacherName) {
    return NextResponse.json({ error: 'cardId and teacherName required' }, { status: 400 })
  }

  // 이미 미반납 대여가 있으면 반환
  const { data: existing } = await sb.from('card_assignments')
    .select('id')
    .eq('card_id', cardId)
    .is('returned_at', null)
    .maybeSingle()

  if (existing) {
    return NextResponse.json({ error: '이미 누군가 사용 중인 카드입니다' }, { status: 409 })
  }

  const { data, error } = await sb.from('card_assignments').insert({
    card_id: cardId,
    teacher_name: teacherName,
    taken_at: new Date().toISOString(),
    note: note || '',
  }).select().single()

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data, { status: 201 })
}

// 반납 처리
export async function PATCH(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()
  const { id } = body

  if (!id) return NextResponse.json({ error: 'id required' }, { status: 400 })

  const { error } = await sb.from('card_assignments')
    .update({ returned_at: new Date().toISOString() })
    .eq('id', id)

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json({ ok: true })
}
