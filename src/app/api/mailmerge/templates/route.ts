import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'

// ─── 템플릿 목록 ───
export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const id = searchParams.get('id')
  const year = searchParams.get('year')
  const type = searchParams.get('type')
  const sb = getServiceSupabase()

  if (id) {
    const { data, error } = await sb.from('mailmerge_templates')
      .select('*, runs:mailmerge_runs(*)')
      .eq('id', id)
      .single()
    if (error) return NextResponse.json({ error: error.message }, { status: 500 })
    return NextResponse.json(data)
  }

  let query = sb.from('mailmerge_templates')
    .select('*')
    .order('year', { ascending: false })
    .order('created_at', { ascending: false })

  if (year) query = query.eq('year', parseInt(year))
  if (type) query = query.eq('document_type', type)

  const { data, error } = await query
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data || [])
}

// ─── 템플릿 생성 ───
export async function POST(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()
  const { title, description, documentType, year, fields, templateFilePath } = body

  if (!title) {
    return NextResponse.json({ error: 'title is required' }, { status: 400 })
  }

  const { data, error } = await sb.from('mailmerge_templates').insert({
    title,
    description: description || null,
    document_type: documentType || 'general',
    year: year || new Date().getFullYear(),
    fields: fields || [],
    template_file_path: templateFilePath || null,
  }).select().single()

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data, { status: 201 })
}

// ─── 템플릿 수정 ───
export async function PATCH(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()
  const { id, ...updates } = body

  if (!id) {
    return NextResponse.json({ error: 'id is required' }, { status: 400 })
  }

  const fieldMap: Record<string, string> = {
    title: 'title',
    description: 'description',
    documentType: 'document_type',
    document_type: 'document_type',
    year: 'year',
    fields: 'fields',
    templateFilePath: 'template_file_path',
    template_file_path: 'template_file_path',
    isActive: 'is_active',
    is_active: 'is_active',
  }

  const dbUpdates: Record<string, any> = { updated_at: new Date().toISOString() }
  for (const [key, val] of Object.entries(updates)) {
    const dbKey = fieldMap[key]
    if (dbKey) dbUpdates[dbKey] = val
  }

  const { data, error } = await sb.from('mailmerge_templates')
    .update(dbUpdates)
    .eq('id', id)
    .select()
    .single()

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data)
}

// ─── 템플릿 삭제 ───
export async function DELETE(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const id = searchParams.get('id')
  if (!id) return NextResponse.json({ error: 'id is required' }, { status: 400 })

  const sb = getServiceSupabase()
  const { error } = await sb.from('mailmerge_templates').delete().eq('id', id)
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json({ success: true })
}
