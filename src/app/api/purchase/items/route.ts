import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'

// ─── 물품 목록 조회 + 신청 집계 포함 ───
export async function GET() {
  const sb = getServiceSupabase()

  const { data: items, error } = await sb.from('purchase_items')
    .select('*')
    .eq('is_active', true)
    .order('category', { ascending: true })
    .order('name', { ascending: true })

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })

  // 각 물품별로 신청 집계
  const itemsWithStats = await Promise.all((items || []).map(async (item: any) => {
    const { data: requests } = await sb.from('purchase_requests')
      .select('quantity')
      .eq('item_id', item.id)

    const totalQuantity = (requests || []).reduce((sum, r) => sum + (r.quantity || 0), 0)
    const totalAmount = totalQuantity * item.unit_price

    return {
      id: item.id,
      name: item.name,
      spec: item.spec || '',
      unitPrice: item.unit_price,
      unit: item.unit || 'EA',
      link: item.link || '',
      description: item.description || '',
      category: item.category || '기타',
      isActive: item.is_active,
      createdAt: item.created_at,
      updatedAt: item.updated_at,
      requestCount: (requests || []).length,
      totalQuantity,
      totalAmount,
    }
  }))

  return NextResponse.json(itemsWithStats)
}

// ─── 물품 등록 ───
export async function POST(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()
  const { name, spec, unitPrice, unit, link, description, category } = body

  if (!name) {
    return NextResponse.json({ error: '물품명은 필수입니다' }, { status: 400 })
  }

  const { data, error } = await sb.from('purchase_items').insert({
    name,
    spec: spec || '',
    unit_price: unitPrice || 0,
    unit: unit || 'EA',
    link: link || '',
    description: description || '',
    category: category || '기타',
  }).select().single()

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data, { status: 201 })
}