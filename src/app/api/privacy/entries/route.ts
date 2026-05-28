import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'
import { requireAuth, unauthorized } from '@/lib/api-auth'

export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const cycleId = searchParams.get('cycleId')
  const departmentId = searchParams.get('departmentId')
  const submitted = searchParams.get('submitted')

  if (!cycleId) {
    return NextResponse.json({ error: 'cycleId는 필수입니다.' }, { status: 400 })
  }

  const sb = getServiceSupabase()

  let query = sb
    .from('privacy_maintenance_entries')
    .select(`
      *,
      department:privacy_departments(*),
      standard:privacy_file_standards(*)
    `)
    .eq('cycle_id', cycleId)

  if (departmentId) query = query.eq('department_id', departmentId)
  if (submitted !== null) query = query.eq('is_submitted', submitted === 'true')

  const { data, error } = await query
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })

  return NextResponse.json(data || [])
}

export async function POST(req: NextRequest) {
  const session = await requireAuth(req)
  if (!session) return unauthorized()
  const sb = getServiceSupabase()
  const body = await req.json()
  const { cycleId, departmentId, standardId } = body

  if (!cycleId || !departmentId || !standardId) {
    return NextResponse.json(
      { error: 'cycleId, departmentId, standardId는 필수입니다.' },
      { status: 400 }
    )
  }

  const { data, error } = await sb
    .from('privacy_maintenance_entries')
    .insert({
      cycle_id: cycleId,
      department_id: departmentId,
      standard_id: standardId,
      is_submitted: false,
    })
    .select(`
      *,
      department:privacy_departments(*),
      standard:privacy_file_standards(*)
    `)
    .single()

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })

  return NextResponse.json(data, { status: 201 })
}

export async function PATCH(req: NextRequest) {
  const session = await requireAuth(req)
  if (!session) return unauthorized()
  const sb = getServiceSupabase()
  const body = await req.json()
  const {
    id,
    dataSubjectCount,
    hasAnomaly,
    anomalyDescription,
    managerName,
    managerPosition,
    notes,
    submit,
  } = body

  if (!id) {
    return NextResponse.json({ error: 'id는 필수입니다.' }, { status: 400 })
  }

  const updates: Record<string, unknown> = {
    updated_at: new Date().toISOString(),
  }

  if (typeof dataSubjectCount === 'number') updates.data_subject_count = dataSubjectCount
  if (typeof hasAnomaly === 'boolean') updates.has_anomaly = hasAnomaly
  if (anomalyDescription !== undefined) updates.anomaly_description = anomalyDescription
  if (managerName !== undefined) updates.manager_name = managerName
  if (managerPosition !== undefined) updates.manager_position = managerPosition
  if (notes !== undefined) updates.notes = notes

  if (submit === true) {
    updates.is_submitted = true
    updates.submitted_at = new Date().toISOString()
  }

  const { data, error } = await sb
    .from('privacy_maintenance_entries')
    .update(updates)
    .eq('id', id)
    .select(`
      *,
      department:privacy_departments(*),
      standard:privacy_file_standards(*)
    `)
    .single()

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })

  return NextResponse.json(data)
}

export async function DELETE(req: NextRequest) {
  const session = await requireAuth(req)
  if (!session) return unauthorized()
  const sb = getServiceSupabase()
  const body = await req.json()
  const { id } = body

  const { data: entry } = await sb
    .from('privacy_maintenance_entries')
    .select('is_submitted')
    .eq('id', id)
    .single()

  if (!entry) {
    return NextResponse.json({ error: '존재하지 않는 입력입니다.' }, { status: 404 })
  }

  if (entry.is_submitted) {
    return NextResponse.json(
      { error: '제출 완료된 입력은 삭제할 수 없습니다.' },
      { status: 400 }
    )
  }

  const { error } = await sb
    .from('privacy_maintenance_entries')
    .delete()
    .eq('id', id)

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })

  return NextResponse.json({ success: true })
}
