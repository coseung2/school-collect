'use client'

import { useEffect, useState } from 'react'
import type { PrivacyMaintenanceCycle, PrivacyDepartment, PrivacyFileStandard } from '@/lib/types'
import { useAuth } from '@/lib/useAuth'

export default function PrivacyAdminPage() {
  const session = useAuth('admin')
  const [tab, setTab] = useState<'cycle' | 'dept' | 'file'>('cycle')
  const [cycles, setCycles] = useState<PrivacyMaintenanceCycle[]>([])
  const [departments, setDepartments] = useState<PrivacyDepartment[]>([])
  const [standards, setStandards] = useState<PrivacyFileStandard[]>([])

  // 사이클 폼
  const [year, setYear] = useState(new Date().getFullYear())
  const [startDate, setStartDate] = useState('')
  const [endDate, setEndDate] = useState('')

  // 부서 폼
  const [deptName, setDeptName] = useState('')
  const [managerName, setManagerName] = useState('')
  const [managerDeptId, setManagerDeptId] = useState('')

  // 파일 폼
  const [fileName, setFileName] = useState('')
  const [businessArea, setBusinessArea] = useState('')
  const [retentionPeriod, setRetentionPeriod] = useState('')
  const [fileDept, setFileDept] = useState('')

  const loadCycles = () => fetch('/api/privacy/cycles').then(r => r.json()).then(setCycles)
  const loadDepts = () => fetch('/api/privacy/departments').then(r => r.json()).then(setDepartments)
  const loadFiles = () => fetch('/api/privacy/standards').then(r => r.json()).then(setStandards)

  useEffect(() => { loadCycles(); loadDepts(); loadFiles() }, [])

  const createCycle = async () => {
    if (!year || !startDate || !endDate) return alert('모든 항목을 입력해주세요.')
    const res = await fetch('/api/privacy/cycles', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ year, startDate, endDate }),
    })
    if (!res.ok) { const e = await res.json(); alert(e.error); return }
    loadCycles()
    alert(`${year}년 정비 사이클이 생성되었습니다! (표준 파일 13개에 대한 빈 입력 레코드 자동 생성)`)
  }

  const activateCycle = async (id: string) => {
    await fetch('/api/privacy/cycles', {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ id, status: 'active' }),
    })
    loadCycles()
  }

  const deleteCycle = async (id: string) => {
    if (!confirm('정말 삭제하시겠습니까? (초안 상태만 가능)')) return
    await fetch('/api/privacy/cycles', {
      method: 'DELETE',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ id }),
    })
    loadCycles()
  }

  const setManager = async () => {
    if (!managerDeptId || !managerName) return alert('부서와 담당자명을 입력해주세요.')
    await fetch('/api/privacy/departments', {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ id: managerDeptId, manager: { name: managerName } }),
    })
    loadDepts()
    alert('담당자가 설정되었습니다.')
  }

  const addFile = async () => {
    if (!fileName || !businessArea || !retentionPeriod || !fileDept) return alert('모든 항목을 입력해주세요.')
    await fetch('/api/privacy/standards', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        businessArea: businessArea,
        fileName: fileName,
        retentionPeriod: retentionPeriod,
        department: fileDept,
      }),
    })
    loadFiles()
    setFileName(''); setBusinessArea(''); setRetentionPeriod(''); setFileDept('')
    alert('표준 파일이 추가되었습니다.')
  }

  const tabs = [
    { key: 'cycle', label: '🔄 정비 사이클' },
    { key: 'dept', label: '🏢 부서/담당자' },
    { key: 'file', label: '📄 표준 파일 목록' },
  ] as const

  return (
    <div>
      <h1 className="text-2xl font-bold mb-6">⚙️ 개인정보파일 정비 관리</h1>

      {/* 탭 */}
      <div className="flex border-b border-gray-200 mb-6">
        {tabs.map(t => (
          <button
            key={t.key}
            className={`px-4 py-2 text-sm border-b-2 -mb-px ${tab === t.key ? 'border-blue-500 font-bold text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700'}`}
            onClick={() => setTab(t.key)}
          >
            {t.label}
          </button>
        ))}
      </div>

      {/* 사이클 탭 */}
      {tab === 'cycle' && (
        <div>
          <div className="bg-white border border-gray-200 p-4 mb-6">
            <h2 className="font-bold mb-3">새 정비 사이클 생성</h2>
            <div className="grid grid-cols-3 gap-3 mb-3">
              <div>
                <label className="text-xs text-gray-500 block mb-1">연도</label>
                <input type="number" className="w-full border border-gray-300 px-3 py-1.5 text-sm" value={year} onChange={e => setYear(parseInt(e.target.value))} />
              </div>
              <div>
                <label className="text-xs text-gray-500 block mb-1">시작일</label>
                <input type="date" className="w-full border border-gray-300 px-3 py-1.5 text-sm" value={startDate} onChange={e => setStartDate(e.target.value)} />
              </div>
              <div>
                <label className="text-xs text-gray-500 block mb-1">마감일</label>
                <input type="date" className="w-full border border-gray-300 px-3 py-1.5 text-sm" value={endDate} onChange={e => setEndDate(e.target.value)} />
              </div>
            </div>
            <button className="bg-blue-600 text-white px-4 py-1.5 text-sm hover:bg-blue-700" onClick={createCycle}>
              사이클 생성
            </button>
          </div>

          <div className="space-y-2">
            {cycles.map(c => (
              <div key={c.id} className="bg-white border border-gray-200 p-4 flex items-center justify-between">
                <div>
                  <span className="font-bold">{c.year}년</span>
                  <span className={`ml-2 text-xs px-2 py-0.5 ${c.status === 'completed' ? 'bg-green-50 text-green-600 border border-green-200' : c.status === 'active' ? 'bg-blue-50 text-blue-600 border border-blue-200' : 'bg-gray-50 text-gray-600 border border-gray-200'}`}>
                    {c.status === 'draft' ? '초안' : c.status === 'active' ? '진행중' : '완료'}
                  </span>
                  {c.status !== 'completed' && (
                    <span className="ml-2 text-sm text-gray-500">
                      제출 {c.submittedCount}/{c.entryCount}건 | 부서 {c.submittedDepartments}/{c.totalDepartments}개
                    </span>
                  )}
                </div>
                <div className="flex gap-2">
                  {c.status === 'draft' && (
                    <>
                      <button className="text-sm text-blue-600 hover:underline" onClick={() => activateCycle(c.id)}>진행 시작</button>
                      <button className="text-sm text-red-500 hover:underline" onClick={() => deleteCycle(c.id)}>삭제</button>
                    </>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* 부서/담당자 탭 */}
      {tab === 'dept' && (
        <div>
          <div className="bg-white border border-gray-200 p-4 mb-6">
            <h2 className="font-bold mb-3">담당자 설정</h2>
            <div className="grid grid-cols-2 gap-3 mb-3">
              <div>
                <label className="text-xs text-gray-500 block mb-1">부서</label>
                <select className="w-full border border-gray-300 px-3 py-1.5 text-sm" value={managerDeptId} onChange={e => setManagerDeptId(e.target.value)}>
                  <option value="">선택</option>
                  {departments.map(d => <option key={d.id} value={d.id}>{d.name}</option>)}
                </select>
              </div>
              <div>
                <label className="text-xs text-gray-500 block mb-1">담당자 성명</label>
                <input type="text" className="w-full border border-gray-300 px-3 py-1.5 text-sm" value={managerName} onChange={e => setManagerName(e.target.value)} placeholder="성명 입력" />
              </div>
            </div>
            <button className="bg-blue-600 text-white px-4 py-1.5 text-sm hover:bg-blue-700" onClick={setManager}>담당자 저장</button>
          </div>

          <div className="space-y-2">
            {departments.map(d => (
              <div key={d.id} className="bg-white border border-gray-200 p-4 flex items-center justify-between">
                <div>
                  <span className="font-bold">{d.name}</span>
                  {d.currentManager && (
                    <span className="ml-2 text-sm text-gray-500">
                      담당자: {d.currentManager.name}
                      {d.currentManager.position && ` (${d.currentManager.position})`}
                    </span>
                  )}
                </div>
                <span className="text-xs text-gray-400">{d.files?.length || '-'}개 파일</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* 파일 목록 탭 */}
      {tab === 'file' && (
        <div>
          <div className="bg-white border border-gray-200 p-4 mb-6">
            <h2 className="font-bold mb-3">표준 파일 추가</h2>
            <div className="grid grid-cols-2 gap-3 mb-3">
              <div>
                <label className="text-xs text-gray-500 block mb-1">파일명</label>
                <input type="text" className="w-full border border-gray-300 px-3 py-1.5 text-sm" value={fileName} onChange={e => setFileName(e.target.value)} />
              </div>
              <div>
                <label className="text-xs text-gray-500 block mb-1">업무분야</label>
                <input type="text" className="w-full border border-gray-300 px-3 py-1.5 text-sm" value={businessArea} onChange={e => setBusinessArea(e.target.value)} placeholder="교무/보건/행정실/민원..." />
              </div>
              <div>
                <label className="text-xs text-gray-500 block mb-1">보유기간</label>
                <input type="text" className="w-full border border-gray-300 px-3 py-1.5 text-sm" value={retentionPeriod} onChange={e => setRetentionPeriod(e.target.value)} placeholder="준영구/5년/10년..." />
              </div>
              <div>
                <label className="text-xs text-gray-500 block mb-1">담당부서</label>
                <select className="w-full border border-gray-300 px-3 py-1.5 text-sm" value={fileDept} onChange={e => setFileDept(e.target.value)}>
                  <option value="">선택</option>
                  {departments.map(d => <option key={d.id} value={d.name}>{d.name}</option>)}
                </select>
              </div>
            </div>
            <button className="bg-blue-600 text-white px-4 py-1.5 text-sm hover:bg-blue-700" onClick={addFile}>파일 추가</button>
          </div>

          <div className="bg-white border border-gray-200">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-gray-200 bg-gray-50">
                  <th className="text-left px-4 py-2 font-medium">업무분야</th>
                  <th className="text-left px-4 py-2 font-medium">파일명</th>
                  <th className="text-left px-4 py-2 font-medium">보유기간</th>
                  <th className="text-left px-4 py-2 font-medium">담당부서</th>
                </tr>
              </thead>
              <tbody>
                {standards.map(s => (
                  <tr key={s.id} className="border-b border-gray-100">
                    <td className="px-4 py-2">{s.businessArea}</td>
                    <td className="px-4 py-2">{s.fileName}</td>
                    <td className="px-4 py-2">{s.retentionPeriod}</td>
                    <td className="px-4 py-2">{s.department}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  )
}
