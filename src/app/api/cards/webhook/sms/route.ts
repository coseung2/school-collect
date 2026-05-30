import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'

/**
 * SMS 웹훅 엔드포인트
 * 
 * 문자 포워딩 서비스(예: Twilio, 문자수신 API)에서
 * 카드 결제 알림 문자를 받아 자동 등록합니다.
 * 
 * 예상 포맷:
 * {
 *   "from": "02-1234",
 *   "text": "[신한카드] 1234-****-****-5678 승인
 *             삼성전자포항점 35,000원 일시불 05/30 14:23",
 *   "received_at": "2026-05-30T14:23:00Z"
 * }
 */
export async function POST(req: NextRequest) {
  const sb = getServiceSupabase()
  let body: any
  
  try {
    body = await req.json()
  } catch {
    // FormData일 수도
    const formData = await req.formData()
    body = Object.fromEntries(formData.entries())
  }

  // SMS 본문에서 카드번호, 금액, 사용처 추출
  const text = body.text || body.body || body.message || ''
  if (!text) {
    return NextResponse.json({ error: 'no text content' }, { status: 400 })
  }

  // 파싱 로직 — 카드사별 문자 포맷에 맞게 조정 필요
  const parsed = parseSms(text)
  if (!parsed) {
    // 파싱 실패해도 200 반환 (재전송 방지)
    return NextResponse.json({ ok: true, parsed: false, message: 'unrecognized format' })
  }

  // 카드 찾기 (카드번호 마지막 4자리로 매칭)
  const cardLast4 = parsed.cardNumber?.replace(/[^0-9]/g, '').slice(-4)
  let cardId = null
  
  if (cardLast4) {
    const { data: cards } = await sb.from('card_expense_cards')
      .select('id, card_number')
      .filter('card_number', 'like', `%${cardLast4}`)
      .limit(1)
    
    if (cards && cards.length > 0) {
      cardId = cards[0].id
    }
  }

  // 현재 카드 대여자 확인
  let teacherName = ''
  if (cardId) {
    const { data: assignment } = await sb.from('card_assignments')
      .select('teacher_name')
      .eq('card_id', cardId)
      .is('returned_at', null)
      .maybeSingle()
    if (assignment) {
      teacherName = assignment.teacher_name
    }
  }

  // 지출 내역 등록
  const { error } = await sb.from('card_expenses').insert({
    card_id: cardId || '00000000-0000-0000-0000-000000000000',
    teacher_name: teacherName || (body.from ? `${body.from}` : ''),
    amount: parsed.amount || 0,
    merchant: parsed.merchant || '알수없음',
    merchant_category: parsed.category || '',
    expense_date: parsed.date || new Date().toISOString(),
    memo: `[SMS] ${parsed.raw}`,
    source: 'sms_webhook',
  })

  if (error) {
    console.error('SMS webhook insert error:', error)
  }

  return NextResponse.json({ ok: true, parsed: true, cardMatched: !!cardId })
}

/** 카드 결제 SMS 파싱 */
function parseSms(text: string): {
  amount: number | null
  merchant: string | null
  cardNumber: string | null
  category: string | null
  date: string | null
  raw: string
} | null {
  const raw = text.trim()
  
  // 금액 추출: "35,000원" "35000원"
  const amountMatch = raw.match(/([\d,]+)\s*원/)
  const amount = amountMatch
    ? parseInt(amountMatch[1].replace(/,/g, ''))
    : null

  // 카드번호 추출: "1234-****-****-5678" "1234-5678"
  const cardMatch = raw.match(/(\d{4})[-\* ]+(?:\*+[ -]*)+?(\d{4})/)
  const cardNumber = cardMatch ? `${cardMatch[1]}-****-****-${cardMatch[2]}` : null

  // 날짜 추출: "05/30 14:23"
  const dateMatch = raw.match(/(\d{1,2})\/(\d{1,2})(?:[\sT](\d{1,2}):(\d{2}))?/)
  let date = null
  if (dateMatch) {
    const now = new Date()
    const month = parseInt(dateMatch[1])
    const day = parseInt(dateMatch[2])
    const hour = dateMatch[3] ? parseInt(dateMatch[3]) : 0
    const min = dateMatch[4] ? parseInt(dateMatch[4]) : 0
    date = new Date(now.getFullYear(), month - 1, day, hour, min).toISOString()
  }

  // 사용처 추출: 카드명/금액 사이 텍스트
  let merchant = null
  if (cardMatch && amountMatch) {
    const afterCard = raw.slice(raw.indexOf(cardMatch[0]) + cardMatch[0].length)
    const beforeAmount = afterCard.slice(0, afterCard.indexOf(amountMatch[0]))
    merchant = beforeAmount.replace(/[\s\n]+/g, ' ').trim() || null
  }
  if (!merchant && amountMatch) {
    // 대체: 승인/금액 사이
    const beforeAmount = raw.slice(0, raw.indexOf(amountMatch[0]))
    const approvalIdx = beforeAmount.lastIndexOf('승인')
    if (approvalIdx >= 0) {
      merchant = beforeAmount.slice(approvalIdx + 2).trim() || null
    }
  }

  if (!amount && !merchant) return null

  return { amount, merchant, cardNumber, category: null, date, raw }
}
