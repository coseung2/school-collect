'use client'

import { useState, useEffect } from 'react'
import type { TripExpenseRequest } from '@/lib/types'
import { tripStatusLabel, transportLabel, fmtMoney } from '@/lib/types'

export default function TripsPage() {
  const [requests, setRequests] = useState<TripExpenseRequest[]>([])
  const [loading, setLoading] = useState(true)
  const [filter, setFilter] = useState('all')

  const loadData = async () => {
    setLoading(true)
    const params = filter !== 'all' ? `?status=${filter}` : ''
    const res = await fetch('/api/trips/requests' + params)
    setRequests(await res.json())
    setLoading(false)
  }

  useEffect(() => { loadData() }, [filter])

  const changeStatus = async (id: string, status: string) => {
    await fetch('/api/trips/requests', {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ id, status }),
    })
    loadData()
  }

  const getCardForTrip = (r: TripExpenseRequest) => {
    const total = r.fuelCost + r.tollCost + r.parkingCost + r.otherCost
    return total
  }

  if (loading) return <div className="doc-empty">로딩중...</div>

  const totalPending = requests.filter(r => r.status === 'submitted').length
  const totalApproved = requests.filter(r => r.status === 'approved').length
  const approvedSum = requests.filter(r => r.status === 'approved')
    .reduce((s, r) => s + r.totalCost, 0)

  return (
    <div className="doc" style={{ maxWidth: 1000, margin: '0 auto', padding: '24px 28px' }}>
      <div className="toolbar">
        <div>
          <h1 style={{ fontSize: 18, fontWeight: 800, margin: 0 }}>🚗 출장여비 신청</h1>
          <div className="sub" style={{ fontSize: 12, color: 'var(--muted)', marginTop: 2 }}>
            유류대 · 톨비 · 주차비 자동 계산
          </div>
        </div>
        <div className="toolbar-right">
          <a href="/trips/new" className="doc-btn primary" style={{ textDecoration: 'none' }}>
            + 새 신청
          </a>
        </div>
      </div>

      {/* 통계 */}
      <div className="stat-grid">
        <div className="stat-box">
          <div className="num">{totalPending}건</div>
          <div className="label">승인 대기</div>
        </div>
        <div className="stat-box">
          <div className="num">{totalApproved}건</div>
          <div className="label">승인 완료</div>
        </div>
        <div className="stat-box">
          <div className="num">{fmtMoney(approvedSum)}</div>
          <div className="label">승인 총액</div>
        </div>
        <div className="stat-box">
          <div className="num">{requests.length}건</div>
          <div className="label">전체 신청</div>
        </div>
      </div>

      {/* 필터 */}
      <div className="toolbar" style={{ borderTop: '1px solid var(--line)', paddingTop: 12 }}>
        <div style={{ display: 'flex', gap: 6 }}>
          {['all', 'draft', 'submitted', 'approved', 'rejected'].map(s => (
            <button key={s}
              className={`doc-btn ${filter === s ? 'primary' : 'ghost'}`}
              style={{ padding: '4px 12px', fontSize: 12 }}
              onClick={() => setFilter(s)}>
              {s === 'all' ? '전체' : tripStatusLabel(s)}
            </button>
          ))}
        </div>
      </div>

      {/* 목록 */}
      {requests.length === 0 ? (
        <div className="doc-empty" style={{ marginTop: 20 }}>
          <div style={{ fontSize: 28, marginBottom: 8 }}>🧾</div>
          출장여비 신청 내역이 없습니다
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 10, marginTop: 12 }}>
          {requests.map(r => (
            <div key={r.id} className="doc" style={{
              padding: '16px 20px',
              borderLeft: r.status === 'approved' ? '4px solid var(--green)'
                : r.status === 'rejected' ? '4px solid var(--red)'
                : r.status === 'submitted' ? '4px solid var(--blue)'
                : '4px solid var(--line)',
            }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                <div style={{ flex: 1 }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
                    <span style={{ fontWeight: 700, fontSize: 15 }}>{r.destination}</span>
                    <span className={`doc-tag ${r.status === 'approved' ? 'green' : r.status === 'rejected' ? 'red' : r.status === 'submitted' ? 'blue' : 'gray'}`}
                      style={{ fontSize: 10 }}>
                      {tripStatusLabel(r.status)}
                    </span>
                  </div>
                  <div style={{ fontSize: 12, color: 'var(--muted)', display: 'flex', gap: 16 }}>
                    <span>📅 {r.tripDate}</span>
                    <span>👤 {r.teacherName}</span>
                    <span>🚗 {transportLabel(r.transport)}</span>
                    <span>📍 {r.purpose}</span>
                  </div>
                </div>
                <div style={{ textAlign: 'right' }}>
                  <div style={{ fontWeight: 800, fontSize: 18, color: 'var(--ink)' }}>
                    {fmtMoney(r.totalCost)}
                  </div>
                  <div style={{ fontSize: 11, color: 'var(--muted)', marginTop: 2 }}>
                    유류 {fmtMoney(r.fuelCost)} + 톨비 {fmtMoney(r.tollCost)} + 주차 {fmtMoney(r.parkingCost)}
                  </div>
                </div>
              </div>
              <div style={{ display: 'flex', gap: 6, marginTop: 10, justifyContent: 'flex-end' }}>
                {r.status === 'draft' && (
                  <button className="doc-btn primary" style={{ padding: '4px 12px', fontSize: 12 }}
                    onClick={() => changeStatus(r.id, 'submitted')}>
                    제출하기
                  </button>
                )}
                {r.status === 'submitted' && (
                  <>
                    <button className="doc-btn green" style={{ padding: '4px 12px', fontSize: 12 }}
                      onClick={() => changeStatus(r.id, 'approved')}>
                      승인
                    </button>
                    <button className="doc-btn danger" style={{ padding: '4px 12px', fontSize: 12 }}
                      onClick={() => changeStatus(r.id, 'rejected')}>
                      반려
                    </button>
                  </>
                )}
                {(r.status === 'approved' || r.status === 'rejected') && r.receiptImageUrl && (
                  <button className="doc-btn" style={{ padding: '4px 12px', fontSize: 12 }}
                    onClick={() => window.open(r.receiptImageUrl, '_blank')}>
                    🧾 영수증
                  </button>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
