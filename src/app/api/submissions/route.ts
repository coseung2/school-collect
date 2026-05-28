import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'

// ─── 제출 목록 조회 (특정 수합) ───
export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const runId = searchParams.get('runId')
  const sb = getServiceSupabase()

  if (!runId) {
    return NextResponse.json({ error: 'runId required' }, { status: 400 })
  }

  const { data, error } = await sb.from('collect_submissions')
    .select(`
      *,
      answers:collect_answers(*)
    `)
    .eq('run_id', runId)
    .order('grade', { ascending: true })
    .order('class_num', { ascending: true })

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data)
}

// ─── 제출 (입력) ───
export async function POST(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()
  const { runId, grade, classNum, submitter, answers } = body

  if (!runId) {
    return NextResponse.json({ error: 'runId required' }, { status: 400 })
  }

  // 기존 제출 확인 (중복 방지)
  const { data: existing } = await sb.from('collect_submissions')
    .select('id')
    .eq('run_id', runId)
    .eq('grade', grade)
    .eq('class_num', classNum)
    .maybeSingle()

  let submissionId: string

  if (existing) {
    // 업데이트: 기존 제출 수정
    const { error: uErr } = await sb.from('collect_submissions')
      .update({ submitter, updated_at: new Date().toISOString() })
      .eq('id', existing.id)
    if (uErr) return NextResponse.json({ error: uErr.message }, { status: 500 })
    submissionId = existing.id

    // 기존 답변 삭제
    await sb.from('collect_answers').delete().eq('submission_id', submissionId)
  } else {
    // 새 제출
    const { data: sub, error: sErr } = await sb.from('collect_submissions').insert({
      run_id: runId,
      grade,
      class_num: classNum,
      submitter,
    }).select().single()

    if (sErr) return NextResponse.json({ error: sErr.message }, { status: 500 })
    submissionId = sub.id
  }

  // 답변 저장
  if (answers && answers.length > 0) {
    const answerRows = answers.map((a: any) => ({
      submission_id: submissionId,
      field_id: a.fieldId,
      value: String(a.value ?? ''),
    }))
    const { error: aErr } = await sb.from('collect_answers').insert(answerRows)
    if (aErr) return NextResponse.json({ error: aErr.message }, { status: 500 })
  }

  return NextResponse.json({ id: submissionId }, { status: existing ? 200 : 201 })
}
