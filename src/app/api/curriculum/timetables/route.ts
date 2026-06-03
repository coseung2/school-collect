import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'
const sb = getServiceSupabase()

export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const yearId = searchParams.get('yearId')
  const term = searchParams.get('term')
  const sourceType = searchParams.get('sourceType') || 'base'
  const grade = searchParams.get('grade')
  const classNo = searchParams.get('classNo')

  let query = sb.from(sourceType === 'semester' ? 'timetable_semester' : 'timetable_base')
    .select('*, subject:subjects(*)')
    .eq('year_id', yearId)
  if (sourceType === 'semester' && term) query = query.eq('term', parseInt(term))
  if (grade) query = query.eq('grade', parseInt(grade))
  if (classNo) query = query.eq('class_no', parseInt(classNo))
  
  const { data, error } = await query
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data)
}

export async function POST(req: NextRequest) {
  const body = await req.json()
  const table = body.sourceType === 'semester' ? 'timetable_semester' : 'timetable_base'
  const entries = body.entries || (Array.isArray(body) ? body : [body])
  
  const { data, error } = await sb.from(table).upsert(entries, {
    onConflict: table === 'timetable_semester' ? 'year_id,term,grade,class_no,weekday,period' : 'year_id,grade,class_no,weekday,period'
  }).select()
  
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data)
}

export async function PATCH(req: NextRequest) {
  const body = await req.json()
  const { id, sourceType, ...updates } = body
  if (!id || !sourceType) return NextResponse.json({ error: 'id and sourceType required' }, { status: 400 })
  const table = sourceType === 'semester' ? 'timetable_semester' : 'timetable_base'
  updates.updated_at = new Date().toISOString()
  const { data, error } = await sb.from(table).update(updates).eq('id', id).select().single()
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data)
}