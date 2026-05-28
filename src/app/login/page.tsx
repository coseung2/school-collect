'use client'

import { useState } from 'react'
import { useRouter } from 'next/navigation'
import { login } from '@/lib/auth-client'

export default function LoginPage() {
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)
  const router = useRouter()

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setLoading(true)
    setError('')

    const { error: loginError } = await login(email, password)
    if (loginError) {
      setError(loginError.message)
      setLoading(false)
      return
    }

    router.push('/')
    router.refresh()
  }

  return (
    <div className="max-w-sm mx-auto mt-20">
      <h1 className="text-2xl font-bold mb-2 text-center">🔐 학교 업무수합</h1>
      <p className="text-sm text-gray-500 text-center mb-8">로그인이 필요합니다</p>

      <form onSubmit={handleSubmit} className="bg-white border border-gray-200 p-6">
        {error && (
          <div className="bg-red-50 border border-red-200 text-red-600 text-sm px-3 py-2 mb-4">
            {error}
          </div>
        )}

        <div className="mb-4">
          <label className="text-xs text-gray-500 block mb-1">이메일</label>
          <input
            type="email"
            className="w-full border border-gray-300 px-3 py-2 text-sm"
            value={email}
            onChange={e => setEmail(e.target.value)}
            placeholder="teacher@school.kr"
            required
          />
        </div>

        <div className="mb-6">
          <label className="text-xs text-gray-500 block mb-1">비밀번호</label>
          <input
            type="password"
            className="w-full border border-gray-300 px-3 py-2 text-sm"
            value={password}
            onChange={e => setPassword(e.target.value)}
            placeholder="••••••••"
            required
          />
        </div>

        <button
          type="submit"
          className="w-full bg-blue-600 text-white py-2 text-sm hover:bg-blue-700 disabled:opacity-50"
          disabled={loading}
        >
          {loading ? '로그인 중...' : '로그인'}
        </button>
      </form>
    </div>
  )
}
