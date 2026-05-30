import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'

export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const teacher = searchParams.get('teacher')
  const status = searchParams.get('status')
  const limit = parseInt(searchParams.get('limit') || '100')
  const sb = getServiceSupabase()

  let query = sb.from('trip_expense_requests')
    .select('*')
    .order('trip_date', { ascending: false })
    .limit(limit)

  if (teacher) query = query.ilike('teacher_name', `%${teacher}%`)
  if (status) query = query.eq('status', status)

  const { data, error } = await query
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data || [])
}

export async function POST(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()

  const { teacherName, department, destination, purpose, transport, tripDate,
    distance, fuelUnitPrice, fuelEfficiency, fuelCost, tollCost, parkingCost,
    otherCost, totalCost, receiptImageUrl, note } = body

  if (!teacherName || !destination || !purpose || !tripDate) {
    return NextResponse.json({ error: 'teacherName, destination, purpose, tripDate required' }, { status: 400 })
  }

  const { data, error } = await sb.from('trip_expense_requests').insert({
    teacher_name: teacherName,
    department: department || '',
    destination,
    purpose,
    transport: transport || 'car',
    trip_date: tripDate,
    distance: distance || 0,
    fuel_unit_price: fuelUnitPrice || 0,
    fuel_efficiency: fuelEfficiency || 10.5,
    fuel_cost: fuelCost || 0,
    toll_cost: tollCost || 0,
    parking_cost: parkingCost || 0,
    other_cost: otherCost || 0,
    total_cost: totalCost || 0,
    receipt_image_url: receiptImageUrl || '',
    note: note || '',
    status: 'draft',
  }).select().single()

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data, { status: 201 })
}

// 상태 변경 (제출/승인/반려)
export async function PATCH(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()
  const { id, status, reviewedBy, reviewNote } = body

  if (!id || !status) {
    return NextResponse.json({ error: 'id and status required' }, { status: 400 })
  }

  const updates: any = { status }
  if (status === 'submitted') updates.submitted_at = new Date().toISOString()
  if (reviewedBy) updates.reviewed_by = reviewedBy
  if (reviewNote !== undefined) updates.review_note = reviewNote

  const { error } = await sb.from('trip_expense_requests')
    .update(updates)
    .eq('id', id)

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json({ ok: true })
}
