import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'
const sb = getServiceSupabase()

export async function POST(req: NextRequest) {
  const { yearId, term } = await req.json()
  if (!yearId || !term) return NextResponse.json({ error: 'yearId and term required' }, { status: 400 })

  // Get base timetable entries
  const { data: baseEntries, error: baseErr } = await sb
    .from('timetable_base')
    .select('year_id, grade, class_no, weekday, period, subject_id, teacher_id')
    .eq('year_id', yearId)

  if (baseErr) return NextResponse.json({ error: baseErr.message }, { status: 500 })
  if (baseEntries.length === 0) return NextResponse.json({ error: '기초시간표가 비어있습니다' }, { status: 400 })

  // Transform to semester entries
  const semesterEntries = baseEntries.map(e => ({
    year_id: e.year_id,
    term,
    grade: e.grade,
    class_no: e.class_no,
    weekday: e.weekday,
    period: e.period,
    subject_id: e.subject_id,
    teacher_id: e.teacher_id,
  }))

  // Remove existing semester entries for this year+term then insert
  await sb.from('timetable_semester').delete().eq('year_id', yearId).eq('term', term)

  const { data, error } = await sb.from('timetable_semester').insert(semesterEntries).select()
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json({ count: data.length, data })
}