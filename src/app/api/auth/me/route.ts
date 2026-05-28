import { NextRequest, NextResponse } from 'next/server'
import { getAuthSession } from '@/lib/auth-api'

export async function GET(req: NextRequest) {
  const auth = req.headers.get('Authorization')
  if (!auth?.startsWith('Bearer ')) {
    return NextResponse.json({ error: '인증 필요' }, { status: 401 })
  }

  const session = await getAuthSession(auth.slice(7))
  if (!session) {
    return NextResponse.json({ error: '유효하지 않은 세션' }, { status: 401 })
  }

  return NextResponse.json(session)
}
