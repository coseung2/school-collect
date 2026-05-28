import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'

// ─── 템플릿 목록 ───
export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const year = searchParams.get('year')
  const sb = getServiceSupabase()

  let query = sb.from('form_templates').select(`
    *,
    fields:form_fields(*)
  `).order('created_at', { ascending: false })

  if (year) query = query.eq('year', parseInt(year))

  const { data, error } = await query
  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data)
}

// ─── 템플릿 생성 ───
export async function POST(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()

  const { title, description, year, fields } = body

  // 템플릿 생성
  const { data: template, error: tErr } = await sb.from('form_templates').insert({
    title,
    description,
    year: year || new Date().getFullYear(),
  }).select().single()

  if (tErr) return NextResponse.json({ error: tErr.message }, { status: 500 })

  // 필드 생성
  if (fields && fields.length > 0) {
    const fieldRows = fields.map((f: any, i: number) => ({
      template_id: template.id,
      label: f.label,
      field_type: f.fieldType || 'text',
      required: f.required || false,
      options: f.options || null,
      order: f.order ?? i,
    }))

    const { error: fErr } = await sb.from('form_fields').insert(fieldRows)
    if (fErr) return NextResponse.json({ error: fErr.message }, { status: 500 })
  }

  // 전체 조회
  const { data: full } = await sb.from('form_templates').select(`
    *,
    fields:form_fields(*)
  `).eq('id', template.id).single()

  return NextResponse.json(full, { status: 201 })
}
