import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'
const sb = getServiceSupabase()

export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const cyId = searchParams.get('curriculumYearId')
  let query = sb.from('curriculum_classes').select('*, homeroom_teacher:sc_teachers(*)').order('grade').order('class_no')
  if (cyId) query = query.eq('curriculum_year_id', cyId)
  const { data, error } = await query
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data)
}

export async function POST(req: NextRequest) {
  const body = await req.json()
  if (Array.isArray(body)) {
    const { data, error } = await sb.from('curriculum_classes').insert(body).select()
    if (error) return NextResponse.json({ error: error.message }, { status: 500 })
    return NextResponse.json(data)
  }
  const { data, error } = await sb.from('curriculum_classes').insert(body).select().single()
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data)
}

export async function DELETE(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const cyId = searchParams.get('curriculumYearId')
  if (!cyId) return NextResponse.json({ error: 'curriculumYearId required' }, { status: 400 })
  const { error } = await sb.from('curriculum_classes').delete().eq('curriculum_year_id', cyId)
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json({ ok: true })
}