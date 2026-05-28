import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'
import type { ExportData } from '@/lib/types'
import { requireAuth, unauthorized } from '@/lib/api-auth'

export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const cycleId = searchParams.get('cycleId')
  const format = searchParams.get('format') || 'json'

  if (!cycleId) {
    return NextResponse.json({ error: 'cycleId는 필수입니다.' }, { status: 400 })
  }

  const sb = getServiceSupabase()

  const { data: cycle } = await sb
    .from('privacy_maintenance_cycles')
    .select('*')
    .eq('id', cycleId)
    .single()

  if (!cycle) {
    return NextResponse.json({ error: '존재하지 않는 사이클입니다.' }, { status: 404 })
  }

  const { data: entries } = await sb
    .from('privacy_maintenance_entries')
    .select(`
      *,
      department:privacy_departments(name),
      standard:privacy_file_standards(*)
    `)
    .eq('cycle_id', cycleId)

  if (!entries || entries.length === 0) {
    return NextResponse.json({ error: '입력 데이터가 없습니다.' }, { status: 404 })
  }

  const exportData: ExportData[] = entries.map((entry) => ({
    fileName: entry.standard?.file_name || '',
    businessArea: entry.standard?.business_area || '',
    dataSubjectCount: entry.data_subject_count || 0,
    retentionPeriod: entry.standard?.retention_period || '',
    hasAnomaly: entry.has_anomaly || false,
    anomalyDescription: entry.anomaly_description || null,
    department: entry.department?.name || entry.standard?.department || '',
    managerName: entry.manager_name || null,
    notes: entry.notes || null,
  }))

  if (format === 'csv') {
    const headers = [
      '파일명',
      '업무분야',
      '정보주체 수',
      '보유기간',
      '이상유무',
      '이상 내용',
      '담당부서',
      '담당자',
      '비고',
    ]

    const rows = exportData.map((d) => [
      d.fileName,
      d.businessArea,
      d.dataSubjectCount.toString(),
      d.retentionPeriod,
      d.hasAnomaly ? '있음' : '없음',
      d.anomalyDescription || '',
      d.department,
      d.managerName || '',
      d.notes || '',
    ])

    const csvContent = [
      headers.join(','),
      ...rows.map((row) =>
        row.map((cell) => `"${String(cell).replace(/"/g, '""')}"`).join(',')
      ),
    ].join('\n')

    await sb.from('privacy_maintenance_history').insert({
      cycle_id: cycleId,
      year: cycle.year,
      export_data: exportData,
      submitted_by: null,
    })

    return new NextResponse(csvContent, {
      status: 200,
      headers: {
        'Content-Type': 'text/csv; charset=utf-8',
        'Content-Disposition': `attachment; filename="개인정보파일정비_${cycle.year}년.csv"`,
      },
    })
  }

  return NextResponse.json({
    year: cycle.year,
    exportDate: new Date().toISOString(),
    data: exportData,
  })
}

export async function POST(req: NextRequest) {
  const session = await requireAuth(req, 'admin')
  if (!session) return unauthorized()
  const sb = getServiceSupabase()
  const body = await req.json()
  const { cycleId, submittedBy } = body

  if (!cycleId) {
    return NextResponse.json({ error: 'cycleId는 필수입니다.' }, { status: 400 })
  }

  const { data: cycle } = await sb
    .from('privacy_maintenance_cycles')
    .select('*')
    .eq('id', cycleId)
    .single()

  if (!cycle) {
    return NextResponse.json({ error: '존재하지 않는 사이클입니다.' }, { status: 404 })
  }

  const { data: entries } = await sb
    .from('privacy_maintenance_entries')
    .select(`
      *,
      department:privacy_departments(name),
      standard:privacy_file_standards(*)
    `)
    .eq('cycle_id', cycleId)

  const exportData: ExportData[] = (entries || []).map((entry) => ({
    fileName: entry.standard?.file_name || '',
    businessArea: entry.standard?.business_area || '',
    dataSubjectCount: entry.data_subject_count || 0,
    retentionPeriod: entry.standard?.retention_period || '',
    hasAnomaly: entry.has_anomaly || false,
    anomalyDescription: entry.anomaly_description || null,
    department: entry.department?.name || entry.standard?.department || '',
    managerName: entry.manager_name || null,
    notes: entry.notes || null,
  }))

  const { data: history, error: historyError } = await sb
    .from('privacy_maintenance_history')
    .insert({
      cycle_id: cycleId,
      year: cycle.year,
      export_data: exportData,
      submitted_by: submittedBy || null,
      submitted_at: new Date().toISOString(),
    })
    .select()
    .single()

  if (historyError) {
    return NextResponse.json({ error: historyError.message }, { status: 500 })
  }

  await sb
    .from('privacy_maintenance_cycles')
    .update({ status: 'completed', updated_at: new Date().toISOString() })
    .eq('id', cycleId)

  return NextResponse.json(history, { status: 201 })
}
