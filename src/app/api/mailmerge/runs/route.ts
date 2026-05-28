import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'

// ─── 실행 목록 ───
export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const id = searchParams.get('id')
  const templateId = searchParams.get('template_id')
  const status = searchParams.get('status')
  const sb = getServiceSupabase()

  if (id) {
    const { data, error } = await sb.from('mailmerge_runs')
      .select('*, template:mailmerge_templates(*)')
      .eq('id', id)
      .single()
    if (error) return NextResponse.json({ error: error.message }, { status: 500 })
    return NextResponse.json(data)
  }

  let query = sb.from('mailmerge_runs')
    .select('*, template:mailmerge_templates(*)')
    .order('created_at', { ascending: false })

  if (templateId) query = query.eq('template_id', templateId)
  if (status) query = query.eq('status', status)

  const { data, error } = await query
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data || [])
}

// ─── 실행 생성 ───
export async function POST(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()
  const { templateId, title, grade, classNum, studentCount, dataFilePath } = body

  if (!templateId || !title) {
    return NextResponse.json({ error: 'templateId and title are required' }, { status: 400 })
  }

  const { data, error } = await sb.from('mailmerge_runs').insert({
    template_id: templateId,
    title,
    grade: grade || null,
    class_num: classNum || null,
    student_count: studentCount || 0,
    data_file_path: dataFilePath || null,
    status: 'draft',
  }).select().single()

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data, { status: 201 })
}

// ─── 실행 수정 (상태 변경 등) ───
export async function PATCH(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()
  const { id, ...updates } = body

  if (!id) {
    return NextResponse.json({ error: 'id is required' }, { status: 400 })
  }

  const fieldMap: Record<string, string> = {
    title: 'title',
    grade: 'grade',
    classNum: 'class_num',
    class_num: 'class_num',
    studentCount: 'student_count',
    student_count: 'student_count',
    dataFilePath: 'data_file_path',
    data_file_path: 'data_file_path',
    status: 'status',
    corrections: 'corrections',
    outputPath: 'output_path',
    output_path: 'output_path',
    errorMessage: 'error_message',
    error_message: 'error_message',
  }

  const dbUpdates: Record<string, any> = { updated_at: new Date().toISOString() }
  for (const [key, val] of Object.entries(updates)) {
    const dbKey = fieldMap[key]
    if (dbKey) dbUpdates[dbKey] = val
  }

  const { data, error } = await sb.from('mailmerge_runs')
    .update(dbUpdates)
    .eq('id', id)
    .select('*, template:mailmerge_templates(*)')
    .single()

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data)
}

// ─── 실행 삭제 ───
export async function DELETE(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const id = searchParams.get('id')
  if (!id) return NextResponse.json({ error: 'id is required' }, { status: 400 })

  const sb = getServiceSupabase()
  const { error } = await sb.from('mailmerge_runs').delete().eq('id', id)
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json({ success: true })
}
