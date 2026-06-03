import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'
const sb = getServiceSupabase()

export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const yearId = searchParams.get('yearId')
  const lessonId = searchParams.get('lessonId')

  let query = sb.from('cross_topic_logs').select('*, topic:cross_topics(*), lesson:lessons(*)')
  if (yearId) query = query.eq('year_id', yearId)
  if (lessonId) query = query.eq('lesson_id', lessonId)

  const { data, error } = await query
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data)
}

export async function POST(req: NextRequest) {
  const body = await req.json()
  const { data, error } = await sb.from('cross_topic_logs').insert(body).select().single()
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data)
}