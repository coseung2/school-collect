import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'
import { requireAuth, unauthorized } from '@/lib/api-auth'

export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const year = searchParams.get('year')
  const status = searchParams.get('status')
  const sb = getServiceSupabase()

  let query = sb
    .from('privacy_maintenance_cycles')
    .select('*')
    .order('year', { ascending: false })

  if (year) query = query.eq('year', parseInt(year))
  if (status) query = query.eq('status', status)

  const { data, error } = await query
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })

  const cyclesWithStats = await Promise.all((data || []).map(async (cycle) => {
    const { count: totalDepts } = await sb
      .from('privacy_departments')
      .select('*', { count: 'exact', head: true })
      .eq('is_active', true)

    const { data: entries } = await sb
      .from('privacy_maintenance_entries')
      .select('department_id')
      .eq('cycle_id', cycle.id)
      .eq('is_submitted', true)

    const submittedDepts = new Set(entries?.map(e => e.department_id) || [])

    const { count: totalEntries } = await sb
      .from('privacy_maintenance_entries')
      .select('*', { count: 'exact', head: true })
      .eq('cycle_id', cycle.id)

    const { count: submittedEntries } = await sb
      .from('privacy_maintenance_entries')
      .select('*', { count: 'exact', head: true })
      .eq('cycle_id', cycle.id)
      .eq('is_submitted', true)

    return {
      ...cycle,
      totalDepartments: totalDepts || 0,
      submittedDepartments: submittedDepts.size,
      entryCount: totalEntries || 0,
      submittedCount: submittedEntries || 0,
    }
  }))

  return NextResponse.json(cyclesWithStats)
}

export async function POST(req: NextRequest) {
  const session = await requireAuth(req, 'admin')
  if (!session) return unauthorized()
  const sb = getServiceSupabase()
  const body = await req.json()
  const { year, startDate, endDate, description } = body

  const { data: existing } = await sb
    .from('privacy_maintenance_cycles')
    .select('id')
    .eq('year', year)
    .single()

  if (existing) {
    return NextResponse.json(
      { error: `${year}년 정비 사이클이 이미 존재합니다.` },
      { status: 409 }
    )
  }

  const { data, error } = await sb
    .from('privacy_maintenance_cycles')
    .insert({
      year,
      start_date: startDate,
      end_date: endDate,
      status: 'draft',
      description: description || null,
    })
    .select()
    .single()

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })

  // 표준 파일목록 기반으로 빈 입력 레코드 자동 생성
  const { data: standards } = await sb
    .from('privacy_file_standards')
    .select('id, department')
    .eq('is_active', true)

  if (standards && standards.length > 0) {
    const entries = await Promise.all(
      standards.map(async (std) => {
        const { data: dept } = await sb
          .from('privacy_departments')
          .select('id')
          .eq('name', std.department)
          .single()

        if (!dept) return null

        return {
          cycle_id: data.id,
          department_id: dept.id,
          standard_id: std.id,
          is_submitted: false,
        }
      })
    )

    const validEntries = entries.filter(e => e !== null)
    if (validEntries.length > 0) {
      await sb.from('privacy_maintenance_entries').insert(validEntries)
    }
  }

  return NextResponse.json(data, { status: 201 })
}

export async function PATCH(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()
  const { id, status } = body

  if (!id || !status) {
    return NextResponse.json(
      { error: 'id와 status는 필수입니다.' },
      { status: 400 }
    )
  }

  const { data, error } = await sb
    .from('privacy_maintenance_cycles')
    .update({ status, updated_at: new Date().toISOString() })
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

  const { data: cycle } = await sb
    .from('privacy_maintenance_cycles')
    .select('status')
    .eq('id', id)
    .single()

  if (!cycle) {
    return NextResponse.json({ error: '존재하지 않는 사이클입니다.' }, { status: 404 })
  }

  if (cycle.status !== 'draft') {
    return NextResponse.json(
      { error: '초안 상태의 정비 사이클만 삭제할 수 있습니다.' },
      { status: 400 }
    )
  }

  const { error } = await sb
    .from('privacy_maintenance_cycles')
    .delete()
    .eq('id', id)

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })

  return NextResponse.json({ success: true })
}
