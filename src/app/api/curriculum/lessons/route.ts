import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'
const sb = getServiceSupabase()

export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const yearId = searchParams.get('yearId')
  const term = searchParams.get('term')
  const grade = searchParams.get('grade')
  const classNo = searchParams.get('classNo')
  const subjectId = searchParams.get('subjectId')

  let query = sb.from('lessons').select('*').order('lesson_no')
  if (yearId) query = query.eq('year_id', yearId)
  if (term) query = query.eq('term', parseInt(term))
  if (grade) query = query.eq('grade', parseInt(grade))
  if (classNo) query = query.eq('class_no', parseInt(classNo))
  if (subjectId) query = query.eq('subject_id', subjectId)

  const { data, error } = await query
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data)
}

export async function POST(req: NextRequest) {
  const body = await req.json()
  const { data, error } = await sb.from('lessons').insert(body).select().single()
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data)
}

export async function PATCH(req: NextRequest) {
  const body = await req.json()
  const { id, ...updates } = body
  if (!id) return NextResponse.json({ error: 'id required' }, { status: 400 })
  updates.updated_at = new Date().toISOString()
  const { data, error } = await sb.from('lessons').update(updates).eq('id', id).select().single()
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data)
}

export async function DELETE(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const id = searchParams.get('id')
  if (!id) return NextResponse.json({ error: 'id required' }, { status: 400 })
  const { error } = await sb.from('lessons').delete().eq('id', id)
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json({ ok: true })
}