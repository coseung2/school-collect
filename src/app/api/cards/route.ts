import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'

export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const active = searchParams.get('active')
  const sb = getServiceSupabase()

  let query = sb.from('card_expense_cards').select(`
    *,
    assignments:card_assignments!inner(*)
  `).order('card_name')

  if (active === 'true') query = query.eq('is_active', true)

  const { data, error } = await query
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })

  // 현재 수령자가 누군지 계산
  const cards = (data || []).map((card: any) => {
    const activeAssign = card.assignments?.find((a: any) => !a.returned_at)
    return {
      ...card,
      currentHolder: activeAssign?.teacher_name || null,
    }
  })

  return NextResponse.json(cards)
}

export async function POST(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()

  const { error } = await sb.from('card_expense_cards').insert({
    card_name: body.cardName,
    card_number: body.cardNumber,
    card_holder: body.cardHolder || '',
    issuing_bank: body.issuingBank || '',
    is_active: true,
  })

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json({ ok: true }, { status: 201 })
}

export async function PATCH(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()
  const { id, ...updates } = body

  if (!id) return NextResponse.json({ error: 'id required' }, { status: 400 })

  const dbUpdates: any = {}
  if (updates.cardName !== undefined) dbUpdates.card_name = updates.cardName
  if (updates.cardNumber !== undefined) dbUpdates.card_number = updates.cardNumber
  if (updates.cardHolder !== undefined) dbUpdates.card_holder = updates.cardHolder
  if (updates.issuingBank !== undefined) dbUpdates.issuing_bank = updates.issuingBank
  if (updates.isActive !== undefined) dbUpdates.is_active = updates.isActive

  const { error } = await sb.from('card_expense_cards')
    .update(dbUpdates)
    .eq('id', id)

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json({ ok: true })
}
