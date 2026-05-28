'use client'

import { useEffect, useState } from 'react'
import { useRouter } from 'next/navigation'
import { getClientSession } from '@/lib/auth-client'
import type { AuthSession } from '@/lib/types'

export function useAuth(requiredRole?: 'admin' | 'department_head' | 'teacher') {
  const [session, setSession] = useState<AuthSession | null | 'loading'>('loading')
  const router = useRouter()

  useEffect(() => {
    getClientSession().then(s => {
      if (!s) {
        router.push('/login')
        return
      }
      if (requiredRole === 'admin' && s.teacher.role !== 'admin') {
        router.push('/')
        return
      }
      setSession(s)
    })
  }, [])

  return session
}
