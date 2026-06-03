import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'
const sb = getServiceSupabase()

export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const yearId = searchParams.get('yearId')

  if (!yearId) return NextResponse.json({ error: 'yearId required' }, { status: 400 })

  const results: any[] = []

  // 1. Check curriculum_hours exist
  const { count: hoursCount } = await sb.from('curriculum_hours').select('*', { count: 'exact', head: true }).eq('year_id', yearId)
  if (!hoursCount || hoursCount === 0) {
    results.push({ field: '시수편제', status: 'missing', message: '시수편제가 입력되지 않았습니다' })
  }

  // 2. Check calendar events
  const { count: calCount } = await sb.from('calendar_events').select('*', { count: 'exact', head: true }).eq('year_id', yearId)
  if (!calCount || calCount === 0) {
    results.push({ field: '학사일정', status: 'missing', message: '학사일정이 입력되지 않았습니다' })
  }

  // 3. Check timetable_base
  const { count: baseCount } = await sb.from('timetable_base').select('*', { count: 'exact', head: true }).eq('year_id', yearId)
  if (!baseCount || baseCount === 0) {
    results.push({ field: '기초시간표', status: 'missing', message: '기초시간표가 작성되지 않았습니다' })
  }

  // 4. Check lessons
  const { count: lessonCount } = await sb.from('lessons').select('*', { count: 'exact', head: true }).eq('year_id', yearId)
  if (!lessonCount || lessonCount === 0) {
    results.push({ field: '교과 진도표', status: 'missing', message: '교과 진도표가 작성되지 않았습니다' })
  }

  // 5. Check cross_topics
  const { count: ctCount } = await sb.from('cross_topics').select('*', { count: 'exact', head: true }).eq('year_id', yearId)
  if (!ctCount || ctCount === 0) {
    results.push({ field: '범교과주제', status: 'missing', message: '범교과주제가 등록되지 않았습니다' })
  }

  const total = 5
  const missing = results.length
  const completeness = total > 0 ? Math.round(((total - missing) / total) * 100) : 0

  return NextResponse.json({
    yearId,
    total,
    missing,
    completeness,
    items: results,
  })
}