import { NextRequest, NextResponse } from 'next/server'
import { getAuthSession } from '@/lib/auth-api'

/**
 * API 라우트에서 Bearer token 검증
 * requiredRole이 있으면 해당 권한도 확인
 * 실패 시 {error, status} 반환, 성공 시 session 반환
 */
export async function requireAuth(req: NextRequest, requiredRole?: 'admin' | 'department_head' | 'teacher') {
  const auth = req.headers.get('Authorization')
  if (!auth?.startsWith('Bearer ')) {
    return null
  }
  const session = await getAuthSession(auth.slice(7))
  if (!session) return null
  if (requiredRole === 'admin' && session.teacher.role !== 'admin') return null
  return session
}

/**
 * 인증 실패 응답
 */
export function unauthorized(message = '인증이 필요합니다.') {
  return NextResponse.json({ error: message }, { status: 401 })
}

export function forbidden(message = '권한이 없습니다.') {
  return NextResponse.json({ error: message }, { status: 403 })
}
