'use client'

import { useState, useEffect } from 'react'
import type { TripDistanceStd, TripExpenseRequest } from '@/lib/types'
import { fmtMoney } from '@/lib/types'

export default function TripsAdminPage() {
  const [distances, setDistances] = useState<TripDistanceStd[]>([])
  const [requests, setRequests] = useState<TripExpenseRequest[]>([])
  const [loading, setLoading] = useState(true)
  const [showModal, setShowModal] = useState(false)
  const [form, setForm] = useState({ region: '', baseDistance: 0, roundTrip: true })

  const loadData = async () => {
    setLoading(true)
    const [distRes, reqRes] = await Promise.all([
      fetch('/api/trips/distances'),
      fetch('/api/trips/requests?limit=200'),
    ])
    setDistances(await distRes.json())
    setRequests(await reqRes.json())
    setLoading(false)
  }

  useEffect(() => { loadData() }, [])

  const addDistance = async () => {
    if (!form.region || !form.baseDistance) return
    await fetch('/api/trips/distances', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(form),
    })
    setShowModal(false)
    setForm({ region: '', baseDistance: 0, roundTrip: true })
    loadData()
  }

  const deleteDistance = async (id: string) => {
    if (!confirm('삭제할까요?')) return
    await fetch(`/api/trips/distances?id=${id}`, { method: 'DELETE' })
    loadData()
  }

  const changeStatus = async (id: string, status: string) => {
    await fetch('/api/trips/requests', {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ id, status }),
    })
    loadData()
  }

  if (loading) return <div className="doc-empty">로딩중...</div>

  return (
    <div className="doc" style={{ maxWidth: 1000, margin: '0 auto', padding: '24px 28px' }}>
      <div className="toolbar">
        <h1 style={{ fontSize: 18, fontWeight: 800, margin: 0 }}>🚗 출장여비 관리</h1>
        <div className="toolbar-right">
          <a href="/trips" className="doc-link" style={{ fontSize: 13 }}>← 신청 목록</a>
        </div>
      </div>

      {/* 거리 기준 관리 */}
      <div className="section-head">
        <h2>📏 지역별 거리 기준</h2>
        <div className="toolbar-right">
          <button className="doc-btn" onClick={() => setShowModal(true)}>+ 추가</button>
        </div>
      </div>

      <table className="doc-table">
        <thead>
          <tr>
            <th>지역</th>
            <th>기본거리 (km)</th>
            <th>왕복</th>
            <th>액션</th>
          </tr>
        </thead>
        <tbody>
          {distances.length === 0 ? (
            <tr><td colSpan={4} style={{ textAlign: 'center', padding: 24, color: 'var(--muted)' }}>
              등록된 거리 기준이 없습니다
            </td></tr>
          ) : (
            distances.map(d => (
              <tr key={d.id}>
                <td style={{ fontWeight: 700 }}>{d.region}</td>
                <td>{d.baseDistance} km</td>
                <td>{d.roundTrip ? '✅ 왕복' : '편도'}</td>
                <td>
                  <button className="doc-btn danger" style={{ padding: '2px 8px', fontSize: 11 }}
                    onClick={() => deleteDistance(d.id)}>삭제</button>
                </td>
              </tr>
            ))
          )}
        </tbody>
      </table>

      {/* 승인 대기 목록 */}
      <div className="section-head" style={{ marginTop: 32 }}>
        <h2>⏳ 승인 대기</h2>
      </div>

      {requests.filter(r => r.status === 'submitted').length === 0 ? (
        <div className="doc-empty">승인 대기 신청이 없습니다</div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
          {requests.filter(r => r.status === 'submitted').map(r => (
            <div key={r.id} className="doc" style={{ padding: '14px 18px' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <div>
                  <div style={{ fontWeight: 700 }}>{r.teacherName} — {r.destination}</div>
                  <div style={{ fontSize: 12, color: 'var(--muted)' }}>
                    {r.tripDate} · {r.purpose} · {fmtMoney(r.totalCost)}
                  </div>
                </div>
                <div style={{ display: 'flex', gap: 6 }}>
                  <button className="doc-btn green" style={{ padding: '4px 12px', fontSize: 12 }}
                    onClick={() => changeStatus(r.id, 'approved')}>✅ 승인</button>
                  <button className="doc-btn danger" style={{ padding: '4px 12px', fontSize: 12 }}
                    onClick={() => changeStatus(r.id, 'rejected')}>❌ 반려</button>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* 거리 추가 모달 */}
      {showModal && (
        <div style={{
          position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.3)',
          display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 50,
        }} onClick={() => setShowModal(false)}>
          <div className="doc" style={{ width: 360, padding: '24px 28px' }}
            onClick={e => e.stopPropagation()}>
            <div className="doc-header">
              <h2 style={{ fontSize: 16, margin: 0 }}>거리 기준 추가</h2>
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              <div className="doc-field" style={{ display: 'flex' }}>
                <label>지역명</label>
                <input value={form.region}
                  onChange={e => setForm({...form, region: e.target.value})}
                  placeholder="e.g. 대구" style={{ flex: 1 }} />
              </div>
              <div className="doc-field" style={{ display: 'flex' }}>
                <label>거리</label>
                <input type="number" value={form.baseDistance || ''}
                  onChange={e => setForm({...form, baseDistance: parseInt(e.target.value) || 0})}
                  placeholder="km" style={{ flex: 1 }} />
              </div>
              <div className="doc-field" style={{ display: 'flex' }}>
                <label>왕복</label>
                <select value={form.roundTrip ? 'true' : 'false'}
                  onChange={e => setForm({...form, roundTrip: e.target.value === 'true'})}
                  style={{ flex: 1 }}>
                  <option value="true">왕복</option>
                  <option value="false">편도</option>
                </select>
              </div>
            </div>
            <div style={{ display: 'flex', gap: 8, marginTop: 20, justifyContent: 'flex-end' }}>
              <button className="doc-btn ghost" onClick={() => setShowModal(false)}>취소</button>
              <button className="doc-btn primary" onClick={addDistance}>추가</button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
