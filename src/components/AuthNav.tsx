'use client'

import { useEffect, useState } from 'react'
import Link from 'next/link'
import { usePathname, useRouter } from 'next/navigation'
import { getClientSession, logout } from '@/lib/auth-client'
import type { AuthSession } from '@/lib/types'

export default function AuthNav() {
  const [session, setSession] = useState<AuthSession | null | 'loading'>('loading')
  const pathname = usePathname()
  const router = useRouter()

  useEffect(() => {
    getClientSession().then(s => setSession(s))
  }, [])

  const handleLogout = async () => {
    await logout()
    setSession(null)
    router.push('/login')
    router.refresh()
  }

  // 로그인 페이지면 다른 nav 숨김
  if (pathname === '/login') return null

  if (session === 'loading') {
    return (
      <nav className="bg-white border-b border-gray-200 sticky top-0 z-10">
        <div className="max-w-6xl mx-auto px-4 h-14 flex items-center justify-between">
          <span className="text-lg font-bold text-gray-800">📋 수합</span>
          <span className="text-xs text-gray-400">로딩 중...</span>
        </div>
      </nav>
    )
  }

  if (!session) {
    return (
      <nav className="bg-white border-b border-gray-200 sticky top-0 z-10">
        <div className="max-w-6xl mx-auto px-4 h-14 flex items-center justify-between">
          <div className="flex items-center gap-1 text-sm">
            <Link href="/" className="text-lg font-bold text-gray-800 tracking-tight mr-3">📋 수합</Link>
            <Link href="/cards" className="px-3 py-1.5 text-gray-600 hover:bg-gray-100">💳 카드지출</Link>
            <Link href="/trips" className="px-3 py-1.5 text-gray-600 hover:bg-gray-100">🚗 출장여비</Link>
            <Link href="/purchase" className="px-3 py-1.5 text-gray-600 hover:bg-gray-100">구입신청</Link>
            <Link href="/mailmerge" className="px-3 py-1.5 text-gray-600 hover:bg-gray-100">메일머지</Link>
          </div>
          <Link href="/login" className="text-sm text-blue-600 hover:underline">로그인</Link>
        </div>
      </nav>
    )
  }

  const isAdmin = session.teacher.role === 'admin'
  const depts = session.teacher.departments || []

  return (
    <nav className="bg-white border-b border-gray-200 sticky top-0 z-10">
      <div className="max-w-6xl mx-auto px-4 h-14 flex items-center justify-between">
        <div className="flex items-center gap-1 text-sm">
          <Link href="/" className="text-lg font-bold text-gray-800 tracking-tight mr-3">📋 수합</Link>
          {isAdmin && (
            <>
              <Link href="/admin" className="px-3 py-1.5 text-gray-600 hover:bg-gray-100">수합 관리</Link>
              <Link href="/admin/plan/new" className="px-3 py-1.5 text-gray-600 hover:bg-gray-100">월중계획 등록</Link>
              <Link href="/privacy" className="px-3 py-1.5 text-gray-600 hover:bg-gray-100">개인정보정비</Link>
              <Link href="/privacy/admin" className="px-3 py-1.5 text-gray-600 hover:bg-gray-100">⚙️</Link>
            </>
          )}
          {depts.length > 0 && (
            <Link href={`/privacy/submit?dept=${encodeURIComponent(depts[0])}`} className="px-3 py-1.5 text-gray-600 hover:bg-gray-100">
              개인정보 입력
            </Link>
          )}
          <Link href="/cards" className="px-3 py-1.5 text-gray-600 hover:bg-gray-100">💳 카드지출</Link>
          {isAdmin && (
            <Link href="/cards/admin" className="px-3 py-1.5 text-gray-600 hover:bg-gray-100">⚙️</Link>
          )}
          <Link href="/trips" className="px-3 py-1.5 text-gray-600 hover:bg-gray-100">🚗 출장여비</Link>
          {isAdmin && (
            <Link href="/trips/admin" className="px-3 py-1.5 text-gray-600 hover:bg-gray-100">⚙️</Link>
          )}
          <Link href="/purchase" className="px-3 py-1.5 text-gray-600 hover:bg-gray-100">구입신청</Link>
          {isAdmin && (
            <Link href="/purchase/admin" className="px-3 py-1.5 text-gray-600 hover:bg-gray-100">구입관리</Link>
          )}
          <Link href="/mailmerge" className="px-3 py-1.5 text-gray-600 hover:bg-gray-100">메일머지</Link>
          {isAdmin && (
            <Link href="/admin/teachers" className="px-3 py-1.5 text-gray-600 hover:bg-gray-100">교사 관리</Link>
          )}
        </div>
        <div className="flex items-center gap-2 text-sm">
          <span className="text-gray-500">
            {session.teacher.name}
            {isAdmin && <span className="text-xs ml-1 text-blue-500">(관리자)</span>}
          </span>
          <button onClick={handleLogout} className="text-gray-400 hover:text-gray-600 text-xs">
            로그아웃
          </button>
        </div>
      </div>
    </nav>
  )
}
