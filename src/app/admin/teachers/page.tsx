'use client'

import { useEffect, useState } from 'react'
import { getClientSession } from '@/lib/auth-client'
import type { AuthSession, ScTeacher, TeacherRole } from '@/lib/types'
import { useRouter } from 'next/navigation'
import { roleLabel } from '@/lib/types'

export default function AdminTeachersPage() {
  const [session, setSession] = useState<AuthSession | null | 'loading'>('loading')
  const [teachers, setTeachers] = useState<ScTeacher[]>([])
  const [showForm, setShowForm] = useState(false)
  const [name, setName] = useState('')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [role, setRole] = useState<TeacherRole>('teacher')
  const [selectedDepts, setSelectedDepts] = useState<string[]>([])
  const router = useRouter()

  const allDepts = ['교무부', '행정실', '보건실', '정보부', '사서']

  useEffect(() => {
    getClientSession().then(s => {
      if (!s || s.teacher.role !== 'admin') {
        router.push('/login')
        return
      }
      setSession(s)
      loadTeachers(s)
    })
  }, [])

  const loadTeachers = async (s: AuthSession) => {
    const res = await fetch('/api/teachers', {
      headers: { Authorization: `Bearer ${(await import('@/lib/auth-client')).getSupabaseClient().auth.getSession().then(r => r.data.session?.access_token)}` }
    })
    if (res.ok) setTeachers(await res.json())
  }

  // Simpler approach - get token directly
  const getToken = async () => {
    const { getSupabaseClient } = await import('@/lib/auth-client')
    const { data: { session: s } } = await getSupabaseClient().auth.getSession()
    return s?.access_token
  }

  useEffect(() => {
    if (!session || session === 'loading') return
    ;(async () => {
      const token = await getToken()
      if (!token) return
      const res = await fetch('/api/teachers', {
        headers: { Authorization: `Bearer ${token}` }
      })
      if (res.ok) setTeachers(await res.json())
    })()
  }, [session])

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault()
    const token = await getToken()
    if (!token) return
    const res = await fetch('/api/teachers', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}` },
      body: JSON.stringify({ name, email, password, role, departments: selectedDepts }),
    })
    if (!res.ok) { const err = await res.json(); alert(err.error); return }
    setShowForm(false)
    setName(''); setEmail(''); setPassword(''); setRole('teacher'); setSelectedDepts([])
    const refreshed = await fetch('/api/teachers', { headers: { Authorization: `Bearer ${token}` }})
    if (refreshed.ok) setTeachers(await refreshed.json())
    alert('✅ 교사 계정이 생성되었습니다.')
  }

  const toggleDept = (dept: string) => {
    setSelectedDepts(prev => prev.includes(dept) ? prev.filter(d => d !== dept) : [...prev, dept])
  }

  if (session === 'loading') return <div className="text-center py-12 text-gray-500">로딩 중...</div>
  if (!session) return null

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold">👤 교사 계정 관리</h1>
        <button
          className="bg-blue-600 text-white px-4 py-1.5 text-sm hover:bg-blue-700"
          onClick={() => setShowForm(!showForm)}
        >
          {showForm ? '취소' : '+ 계정 생성'}
        </button>
      </div>

      {showForm && (
        <form onSubmit={handleCreate} className="bg-white border border-gray-200 p-4 mb-6">
          <h2 className="font-bold mb-3">새 교사 계정 생성</h2>
          <div className="grid grid-cols-2 gap-3 mb-3">
            <div>
              <label className="text-xs text-gray-500 block mb-1">이름</label>
              <input type="text" className="w-full border border-gray-300 px-3 py-1.5 text-sm" value={name} onChange={e => setName(e.target.value)} required />
            </div>
            <div>
              <label className="text-xs text-gray-500 block mb-1">이메일 (로그인 ID)</label>
              <input type="email" className="w-full border border-gray-300 px-3 py-1.5 text-sm" value={email} onChange={e => setEmail(e.target.value)} required />
            </div>
            <div>
              <label className="text-xs text-gray-500 block mb-1">임시 비밀번호</label>
              <input type="text" className="w-full border border-gray-300 px-3 py-1.5 text-sm" value={password} onChange={e => setPassword(e.target.value)} required />
            </div>
            <div>
              <label className="text-xs text-gray-500 block mb-1">권한</label>
              <select className="w-full border border-gray-300 px-3 py-1.5 text-sm" value={role} onChange={e => setRole(e.target.value as TeacherRole)}>
                <option value="teacher">교사</option>
                <option value="department_head">부서장</option>
                <option value="admin">관리자</option>
              </select>
            </div>
          </div>
          <div className="mb-3">
            <label className="text-xs text-gray-500 block mb-1">업무 배정 (담당 부서)</label>
            <div className="flex flex-wrap gap-2">
              {allDepts.map(dept => (
                <label key={dept} className="flex items-center gap-1 text-sm border border-gray-200 px-3 py-1 cursor-pointer">
                  <input type="checkbox" checked={selectedDepts.includes(dept)} onChange={() => toggleDept(dept)} />
                  {dept}
                </label>
              ))}
            </div>
          </div>
          <button type="submit" className="bg-blue-600 text-white px-4 py-1.5 text-sm hover:bg-blue-700">
            계정 생성
          </button>
        </form>
      )}

      <div className="bg-white border border-gray-200">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-gray-200 bg-gray-50">
              <th className="text-left px-4 py-2 font-medium">이름</th>
              <th className="text-left px-4 py-2 font-medium">이메일</th>
              <th className="text-left px-4 py-2 font-medium">권한</th>
              <th className="text-left px-4 py-2 font-medium">담당 부서</th>
              <th className="text-left px-4 py-2 font-medium">상태</th>
            </tr>
          </thead>
          <tbody>
            {teachers.map(t => (
              <tr key={t.id} className="border-b border-gray-100">
                <td className="px-4 py-2 font-medium">{t.name}</td>
                <td className="px-4 py-2 text-gray-500">{t.email}</td>
                <td className="px-4 py-2">
                  <span className={`text-xs px-2 py-0.5 border ${
                    t.role === 'admin' ? 'bg-blue-50 text-blue-600 border-blue-200' :
                    t.role === 'department_head' ? 'bg-purple-50 text-purple-600 border-purple-200' :
                    'bg-gray-50 text-gray-600 border-gray-200'
                  }`}>
                    {roleLabel(t.role)}
                  </span>
                </td>
                <td className="px-4 py-2 text-gray-500">
                  {t.departments?.join(', ') || '-'}
                </td>
                <td className="px-4 py-2">
                  {t.isActive
                    ? <span className="text-xs text-green-600">활성</span>
                    : <span className="text-xs text-red-500">비활성</span>}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  )
}
