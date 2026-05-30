import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'

export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const cardId = searchParams.get('cardId')
  const teacher = searchParams.get('teacher')
  const limit = parseInt(searchParams.get('limit') || '100')
  const sb = getServiceSupabase()

  let query = sb.from('card_expenses').select(`
    *,
    card:card_expense_cards(*)
  `).order('expense_date', { ascending: false }).limit(limit)

  if (cardId) query = query.eq('card_id', cardId)
  if (teacher) query = query.ilike('teacher_name', `%${teacher}%`)

  const { data, error } = await query
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data || [])
}

export async function POST(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()

  const { cardId, teacherName, amount, merchant, merchantCategory, expenseDate, receiptImageUrl, memo, source } = body

  if (!cardId || !amount || !merchant) {
    return NextResponse.json({ error: 'cardId, amount, merchant required' }, { status: 400 })
  }

  // 현재 카드 대여자 자동 매칭
  let assignTeacherName = teacherName || ''
  if (!assignTeacherName) {
    const { data: assignment } = await sb.from('card_assignments')
      .select('id, teacher_name')
      .eq('card_id', cardId)
      .is('returned_at', null)
      .maybeSingle()
    if (assignment) {
      assignTeacherName = assignment.teacher_name
    }
  }

  const { data, error } = await sb.from('card_expenses').insert({
    card_id: cardId,
    teacher_name: assignTeacherName,
    amount,
    merchant,
    merchant_category: merchantCategory || '',
    expense_date: expenseDate || new Date().toISOString(),
    receipt_image_url: receiptImageUrl || '',
    memo: memo || '',
    source: source || 'manual',
  }).select().single()

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data, { status: 201 })
}

// 영수증 URL 업데이트
export async function PATCH(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()
  const { id, receiptImageUrl, memo, teacherName } = body

  if (!id) return NextResponse.json({ error: 'id required' }, { status: 400 })

  const updates: any = {}
  if (receiptImageUrl !== undefined) updates.receipt_image_url = receiptImageUrl
  if (memo !== undefined) updates.memo = memo
  if (teacherName !== undefined) updates.teacher_name = teacherName

  const { error } = await sb.from('card_expenses')
    .update(updates)
    .eq('id', id)

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json({ ok: true })
}
