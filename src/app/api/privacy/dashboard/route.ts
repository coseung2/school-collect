import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'
import type { DashboardSummary, DepartmentProgress } from '@/lib/types'

export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  let cycleId = searchParams.get('cycleId')
  const sb = getServiceSupabase()

  if (!cycleId) {
    const { data: latestCycle } = await sb
      .from('privacy_maintenance_cycles')
      .select('*')
      .eq('status', 'active')
      .order('year', { ascending: false })
      .limit(1)
      .single()

    if (!latestCycle) {
      const { data: recentCycle } = await sb
        .from('privacy_maintenance_cycles')
        .select('*')
        .order('year', { ascending: false })
        .limit(1)
        .single()

      if (!recentCycle) {
        return NextResponse.json({ error: '정비 사이클이 없습니다.' }, { status: 404 })
      }
      cycleId = recentCycle.id
    } else {
      cycleId = latestCycle.id
    }
  }

  const { data: cycle } = await sb
    .from('privacy_maintenance_cycles')
    .select('*')
    .eq('id', cycleId)
    .single()

  if (!cycle) {
    return NextResponse.json({ error: '존재하지 않는 사이클입니다.' }, { status: 404 })
  }

  const { data: departments } = await sb
    .from('privacy_departments')
    .select('*')
    .eq('is_active', true)
    .order('display_order')

  const departmentProgress: DepartmentProgress[] = await Promise.all(
    (departments || []).map(async (dept) => {
      const { data: files } = await sb
        .from('privacy_file_standards')
        .select('id')
        .eq('department', dept.name)
        .eq('is_active', true)

      const totalFiles = files?.length || 0

      const { data: entries } = await sb
        .from('privacy_maintenance_entries')
        .select('*, standard:privacy_file_standards(*)')
        .eq('cycle_id', cycleId)
        .eq('department_id', dept.id)

      const submittedFiles = entries?.filter(e => e.is_submitted).length || 0

      const { data: manager } = await sb
        .from('privacy_department_managers')
        .select('*')
        .eq('department_id', dept.id)
        .eq('is_active', true)
        .single()

      return {
        department: { ...dept, currentManager: manager },
        totalFiles,
        submittedFiles,
        pendingFiles: totalFiles - submittedFiles,
        entries: entries || [],
      }
    })
  )

  const { count: totalEntries } = await sb
    .from('privacy_maintenance_entries')
    .select('*', { count: 'exact', head: true })
    .eq('cycle_id', cycleId)

  const { count: submittedEntries } = await sb
    .from('privacy_maintenance_entries')
    .select('*', { count: 'exact', head: true })
    .eq('cycle_id', cycleId)
    .eq('is_submitted', true)

  const { data: recentHistory } = await sb
    .from('privacy_maintenance_history')
    .select('*')
    .order('year', { ascending: false })
    .limit(5)

  const summary: DashboardSummary = {
    cycle: {
      ...cycle,
      totalDepartments: departments?.length || 0,
      submittedDepartments: departmentProgress.filter(d => d.pendingFiles === 0).length,
      entryCount: totalEntries || 0,
      submittedCount: submittedEntries || 0,
    },
    totalDepartments: departments?.length || 0,
    completedDepartments: departmentProgress.filter(d => d.pendingFiles === 0).length,
    totalEntries: totalEntries || 0,
    submittedEntries: submittedEntries || 0,
    pendingEntries: (totalEntries || 0) - (submittedEntries || 0),
    departmentProgress,
    recentHistory: recentHistory || [],
  }

  return NextResponse.json(summary)
}
