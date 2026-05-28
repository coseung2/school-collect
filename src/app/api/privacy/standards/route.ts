import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'
import { requireAuth, unauthorized } from '@/lib/api-auth'

export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const department = searchParams.get('department')
  const active = searchParams.get('active')

  const sb = getServiceSupabase()

  let query = sb
    .from('privacy_file_standards')
    .select('*')
    .order('business_area')

  if (active !== 'false') {
    query = query.eq('is_active', true)
  }
  if (department) {
    query = query.eq('department', department)
  }

  const { data, error } = await query
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })

  return NextResponse.json(data || [])
}

export async function POST(req: NextRequest) {
  const session = await requireAuth(req, 'admin')
  if (!session) return unauthorized()
  const sb = getServiceSupabase()
  const body = await req.json()
  const { businessArea, fileName, retentionPeriod, department, guide } = body

  if (!businessArea || !fileName || !retentionPeriod || !department) {
    return NextResponse.json(
      { error: '업무분야, 파일명, 보유기간, 담당부서는 필수입니다.' },
      { status: 400 }
    )
  }

  const { data, error } = await sb
    .from('privacy_file_standards')
    .insert({
      business_area: businessArea,
      file_name: fileName,
      retention_period: retentionPeriod,
      department,
      guide: guide || null,
    })
    .select()
    .single()

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })

  return NextResponse.json(data, { status: 201 })
}

export async function PATCH(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()
  const { id, ...updates } = body

  if (!id) {
    return NextResponse.json({ error: 'id는 필수입니다.' }, { status: 400 })
  }

  const dbUpdates: Record<string, unknown> = {}
  if (updates.businessArea !== undefined) dbUpdates.business_area = updates.businessArea
  if (updates.fileName !== undefined) dbUpdates.file_name = updates.fileName
  if (updates.retentionPeriod !== undefined) dbUpdates.retention_period = updates.retentionPeriod
  if (updates.department !== undefined) dbUpdates.department = updates.department
  if (updates.guide !== undefined) dbUpdates.guide = updates.guide
  if (updates.isActive !== undefined) dbUpdates.is_active = updates.isActive

  const { data, error } = await sb
    .from('privacy_file_standards')
    .update(dbUpdates)
    .eq('id', id)
    .select()
    .single()

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })

  return NextResponse.json(data)
}

export async function DELETE(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()
  const { id } = body

  const { error } = await sb
    .from('privacy_file_standards')
    .update({ is_active: false })
    .eq('id', id)

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })

  return NextResponse.json({ success: true })
}
