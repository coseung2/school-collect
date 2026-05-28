import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'
import { requireAuth, unauthorized } from '@/lib/api-auth'

export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const cycleId = searchParams.get('cycleId')
  const sb = getServiceSupabase()

  const { data: departments, error } = await sb
    .from('privacy_departments')
    .select('*')
    .eq('is_active', true)
    .order('display_order')

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })

  if (cycleId) {
    const departmentsWithEntries = await Promise.all(
      departments.map(async (dept) => {
        const { data: manager } = await sb
          .from('privacy_department_managers')
          .select('*')
          .eq('department_id', dept.id)
          .eq('is_active', true)
          .single()

        const { data: files } = await sb
          .from('privacy_file_standards')
          .select('*')
          .eq('department', dept.name)
          .eq('is_active', true)

        const { data: entries } = await sb
          .from('privacy_maintenance_entries')
          .select('*, standard:privacy_file_standards(*)')
          .eq('cycle_id', cycleId)
          .eq('department_id', dept.id)

        const submittedCount = entries?.filter(e => e.is_submitted).length || 0

        return {
          ...dept,
          currentManager: manager,
          files: files || [],
          entries: entries || [],
          totalFiles: files?.length || 0,
          submittedFiles: submittedCount,
          pendingFiles: (files?.length || 0) - submittedCount,
        }
      })
    )

    return NextResponse.json(departmentsWithEntries)
  }

  return NextResponse.json(departments)
}

export async function POST(req: NextRequest) {
  const session = await requireAuth(req, 'admin')
  if (!session) return unauthorized()
  const sb = getServiceSupabase()
  const body = await req.json()
  const { name, displayOrder } = body

  if (!name) {
    return NextResponse.json({ error: '부서명은 필수입니다.' }, { status: 400 })
  }

  const { data, error } = await sb
    .from('privacy_departments')
    .insert({
      name,
      display_order: displayOrder || 0,
    })
    .select()
    .single()

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })

  return NextResponse.json(data, { status: 201 })
}

export async function PATCH(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()
  const { id, name, displayOrder, manager } = body

  if (!id) {
    return NextResponse.json({ error: 'id는 필수입니다.' }, { status: 400 })
  }

  if (manager) {
    await sb
      .from('privacy_department_managers')
      .update({ is_active: false })
      .eq('department_id', id)

    const { data, error } = await sb
      .from('privacy_department_managers')
      .insert({
        department_id: id,
        name: manager.name,
        position: manager.position || null,
        phone: manager.phone || null,
        email: manager.email || null,
        is_active: true,
      })
      .select('*')
      .single()

    if (error) return NextResponse.json({ error: error.message }, { status: 500 })

    return NextResponse.json(data)
  }

  const updates: Record<string, unknown> = {}
  if (name) updates.name = name
  if (typeof displayOrder === 'number') updates.display_order = displayOrder

  const { data, error } = await sb
    .from('privacy_departments')
    .update(updates)
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
    .from('privacy_departments')
    .update({ is_active: false })
    .eq('id', id)

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })

  return NextResponse.json({ success: true })
}
