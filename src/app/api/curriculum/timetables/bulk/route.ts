import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'
const sb = getServiceSupabase()

export async function POST(req: NextRequest) {
  const { sourceType, yearId, term, grade, classNo, entries } = await req.json()
  // entries: [{ weekday, period, subjectId }]
  if (!sourceType || !yearId || !grade || classNo === undefined) {
    return NextResponse.json({ error: 'sourceType, yearId, grade, classNo required' }, { status: 400 })
  }

  const table = sourceType === 'semester' ? 'timetable_semester' : 'timetable_base'

  // Delete existing for this grid
  let del = sb.from(table).delete().eq('year_id', yearId).eq('grade', grade).eq('class_no', classNo)
  if (sourceType === 'semester' && term) del = del.eq('term', term)
  await del

  // Insert new
  const inserts = entries
    .filter((e: any) => e.subjectId)
    .map((e: any) => ({
      year_id: yearId,
      ...(sourceType === 'semester' && term ? { term } : {}),
      grade,
      class_no: classNo,
      weekday: e.weekday,
      period: e.period,
      subject_id: e.subjectId,
    }))

  if (inserts.length === 0) return NextResponse.json({ count: 0 })

  const { data, error } = await sb.from(table).insert(inserts).select()
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json({ count: data.length })
}