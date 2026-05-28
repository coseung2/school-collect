'use client'

import { useState, useEffect } from 'react'
import type { PurchaseItem, PurchaseRequest } from '@/lib/types'

export default function PurchasePage() {
  const [items, setItems] = useState<PurchaseItem[]>([])
  const [loading, setLoading] = useState(true)
  const [modalItem, setModalItem] = useState<PurchaseItem | null>(null)
  const [myRequests, setMyRequests] = useState<Map<string, PurchaseRequest>>(new Map())
  const [grade, setGrade] = useState(5)
  const [classNum, setClassNum] = useState(1)
  const [quantity, setQuantity] = useState(1)
  const [submitter, setSubmitter] = useState('')
  const [saving, setSaving] = useState(false)
  const [message, setMessage] = useState('')

  const fetchItems = async () => {
    const res = await fetch('/api/purchase/items')
    const data = await res.json()
    setItems(data)
    setLoading(false)
  }

  const fetchMyRequests = async () => {
    const res = await fetch('/api/purchase/requests')
    const data: PurchaseRequest[] = await res.json()
    const map = new Map<string, PurchaseRequest>()
    data.forEach(r => map.set(r.itemId, r))
    setMyRequests(map)
  }

  useEffect(() => { fetchItems(); fetchMyRequests() }, [])

  const openModal = (item: PurchaseItem) => {
    const existing = myRequests.get(item.id)
    if (existing) {
      setGrade(existing.grade)
      setClassNum(existing.classNum)
      setQuantity(existing.quantity)
      setSubmitter(existing.submitter)
    } else {
      setGrade(5)
      setClassNum(1)
      setQuantity(1)
      setSubmitter('')
    }
    setModalItem(item)
    setMessage('')
  }

  const handleSubmit = async () => {
    if (!modalItem) return
    setSaving(true)
    setMessage('')

    const res = await fetch('/api/purchase/requests', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        itemId: modalItem.id,
        grade,
        classNum,
        quantity,
        submitter,
      }),
    })

    const result = await res.json()
    if (res.ok) {
      setMessage('✅ 신청 완료!')
      await fetchItems()
      await fetchMyRequests()
      setTimeout(() => setModalItem(null), 800)
    } else {
      setMessage('❌ 오류: ' + (result.error || '알 수 없는 오류'))
    }
    setSaving(false)
  }

  const handleDelete = async (itemId: string) => {
    const existing = myRequests.get(itemId)
    if (!existing) return
    if (!confirm('신청을 취소할까요?')) return

    const res = await fetch(`/api/purchase/requests?id=${existing.id}`, { method: 'DELETE' })
    if (res.ok) {
      await fetchItems()
      await fetchMyRequests()
    }
  }

  if (loading) return <div className="doc-empty">로딩중...</div>

  // 카테고리별 그룹
  const categories = [...new Set(items.map(i => i.category))]

  // 금액 포맷
  const fmt = (n: number) => n.toLocaleString('ko-KR') + '원'

  return (
    <div className="doc" style={{ maxWidth: 800, margin: '0 auto', padding: '24px 28px' }}>
      <div className="doc-header">
        <h1>정보화기기 구입 신청</h1>
        <div className="sub">필요한 물품을 선택하고 신청해주세요</div>
      </div>

      {items.length === 0 ? (
        <div className="doc-empty">등록된 물품이 없습니다</div>
      ) : (
        categories.map(cat => {
          const catItems = items.filter(i => i.category === cat)
          return (
            <div key={cat} style={{ marginBottom: 24 }}>
              <div className="section-head">
                <h2>{cat}</h2>
                <span className="meta">{catItems.length}종</span>
              </div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
                {catItems.map(item => {
                  const myReq = myRequests.get(item.id)
                  return (
                    <div key={item.id}
                      className="doc"
                      style={{
                        display: 'flex', alignItems: 'center', gap: 16,
                        padding: '14px 18px', background: myReq ? 'var(--green-weak)' : 'var(--white)'
                      }}>
                      <div style={{ flex: 1, minWidth: 0 }}>
                        <div style={{ fontWeight: 700, fontSize: 14, color: 'var(--ink)' }}>
                          {item.name}
                        </div>
                        {item.spec && (
                          <div style={{ fontSize: 12, color: 'var(--muted)', marginTop: 2 }}>
                            {item.spec}
                          </div>
                        )}
                        <div style={{ display: 'flex', gap: 12, marginTop: 4, fontSize: 12 }}>
                          <span style={{ fontWeight: 600, color: 'var(--blue)' }}>
                            {fmt(item.unitPrice)} / {item.unit}
                          </span>
                          {item.link && (
                            <a href={item.link} target="_blank" rel="noreferrer"
                              className="doc-link" style={{ fontSize: 12 }}>🔗 링크</a>
                          )}
                          {item.requestCount ? (
                            <span style={{ color: 'var(--muted)' }}>
                              {item.requestCount}개 학급 신청중 (총 {item.totalQuantity}EA, {fmt(item.totalAmount || 0)})
                            </span>
                          ) : null}
                        </div>
                      </div>
                      <div style={{ display: 'flex', gap: 6, flexShrink: 0 }}>
                        {myReq ? (
                          <>
                            <span className="doc-tag green">
                              {myReq.grade}-{myReq.classNum} {myReq.quantity}EA 신청완료
                            </span>
                            <button className="doc-btn danger" onClick={() => handleDelete(item.id)}
                              style={{ padding: '4px 10px', fontSize: 12 }}>
                              취소
                            </button>
                          </>
                        ) : (
                          <button className="doc-btn primary" onClick={() => openModal(item)}
                            style={{ padding: '6px 16px', fontSize: 13 }}>
                            신청
                          </button>
                        )}
                      </div>
                    </div>
                  )
                })}
              </div>
            </div>
          )
        })
      )}

      {/* 신청 모달 */}
      {modalItem && (
        <div style={{
          position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.3)',
          display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 50,
        }} onClick={() => setModalItem(null)}>
          <div className="doc" style={{
            width: 380, padding: '24px 28px',
          }} onClick={e => e.stopPropagation()}>
            <div className="doc-header">
              <h2 style={{ fontSize: 16, margin: 0 }}>{modalItem.name}</h2>
              <div className="sub" style={{ marginTop: 4 }}>{fmt(modalItem.unitPrice)} / {modalItem.unit}</div>
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              <div className="doc-field">
                <label>학년</label>
                <select value={grade} onChange={e => setGrade(Number(e.target.value))}>
                  {[1,2,3,4,5,6].map(g => (
                    <option key={g} value={g}>{g}학년</option>
                  ))}
                </select>
              </div>
              <div className="doc-field">
                <label>반</label>
                <select value={classNum} onChange={e => setClassNum(Number(e.target.value))}>
                  {Array.from({length: 8}, (_, i) => i + 1).map(c => (
                    <option key={c} value={c}>{c}반</option>
                  ))}
                </select>
              </div>
              <div className="doc-field">
                <label>개수</label>
                <input type="number" min={1} max={99} value={quantity}
                  onChange={e => setQuantity(Number(e.target.value))} />
              </div>
              <div className="doc-field">
                <label>작성자</label>
                <input value={submitter} onChange={e => setSubmitter(e.target.value)}
                  placeholder="이름 (선택)" />
              </div>
            </div>

            {message && (
              <div style={{ marginTop: 12, fontSize: 13, fontWeight: 600,
                color: message.startsWith('✅') ? 'var(--green)' : 'var(--red)' }}>
                {message}
              </div>
            )}

            <div style={{ display: 'flex', gap: 8, marginTop: 20, justifyContent: 'flex-end' }}>
              <button className="doc-btn ghost" onClick={() => setModalItem(null)}>
                닫기
              </button>
              <button className="doc-btn primary" onClick={handleSubmit} disabled={saving}>
                {saving ? '저장중...' : '신청하기'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}