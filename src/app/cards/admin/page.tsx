'use client'

import { useState, useEffect } from 'react'
import type { CardExpenseCard, CardAssignment } from '@/lib/types'

export default function CardsAdminPage() {
  const [cards, setCards] = useState<CardExpenseCard[]>([])
  const [assignments, setAssignments] = useState<CardAssignment[]>([])
  const [loading, setLoading] = useState(true)
  const [showAddModal, setShowAddModal] = useState(false)
  const [editCard, setEditCard] = useState<CardExpenseCard | null>(null)

  // 등록/수정 폼
  const [form, setForm] = useState({
    cardName: '',
    cardNumber: '',
    cardHolder: '',
    issuingBank: '',
  })

  const loadData = async () => {
    setLoading(true)
    const [cardsRes, assignRes] = await Promise.all([
      fetch('/api/cards'),
      fetch('/api/cards/assignments?limit=100'),
    ])
    setCards(await cardsRes.json())
    setAssignments(await assignRes.json())
    setLoading(false)
  }

  useEffect(() => { loadData() }, [])

  const openAdd = () => {
    setForm({ cardName: '', cardNumber: '', cardHolder: '', issuingBank: '' })
    setEditCard(null)
    setShowAddModal(true)
  }

  const openEdit = (card: CardExpenseCard) => {
    setForm({
      cardName: card.cardName,
      cardNumber: card.cardNumber,
      cardHolder: card.cardHolder,
      issuingBank: card.issuingBank,
    })
    setEditCard(card)
    setShowAddModal(true)
  }

  const submitForm = async () => {
    if (!form.cardName || !form.cardNumber) {
      alert('카드명과 카드번호는 필수입니다')
      return
    }

    const url = editCard ? '/api/cards' : '/api/cards'
    const res = await fetch(url, {
      method: editCard ? 'PATCH' : 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(editCard ? { ...form, id: editCard.id } : form),
    })

    if (res.ok) {
      setShowAddModal(false)
      loadData()
    } else {
      const err = await res.json()
      alert('오류: ' + err.error)
    }
  }

  const toggleActive = async (card: CardExpenseCard) => {
    await fetch('/api/cards', {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ id: card.id, isActive: !card.isActive }),
    })
    loadData()
  }

  if (loading) return <div className="doc-empty">로딩중...</div>

  return (
    <div className="doc" style={{ maxWidth: 900, margin: '0 auto', padding: '24px 28px' }}>
      <div className="toolbar">
        <h1 style={{ fontSize: 18, fontWeight: 800, margin: 0 }}>⚙️ 카드 관리</h1>
        <div className="toolbar-right">
          <a href="/cards" className="doc-link" style={{ fontSize: 13 }}>← 지출 대장</a>
          <button className="doc-btn primary" onClick={openAdd}>+ 새 카드 등록</button>
        </div>
      </div>

      {/* 카드 목록 */}
      <table className="doc-table">
        <thead>
          <tr>
            <th>카드명</th>
            <th>카드번호</th>
            <th>명의자</th>
            <th>은행</th>
            <th>상태</th>
            <th>액션</th>
          </tr>
        </thead>
        <tbody>
          {cards.length === 0 ? (
            <tr><td colSpan={6} style={{ textAlign: 'center', padding: 32, color: 'var(--muted)' }}>
              등록된 카드가 없습니다
            </td></tr>
          ) : (
            cards.map(card => (
              <tr key={card.id} style={{ opacity: card.isActive ? 1 : 0.5 }}>
                <td style={{ fontWeight: 700 }}>{card.cardName}</td>
                <td style={{ fontSize: 12, fontFamily: 'monospace' }}>{card.cardNumber}</td>
                <td>{card.cardHolder || '-'}</td>
                <td>{card.issuingBank || '-'}</td>
                <td>
                  <span className={`doc-tag ${card.isActive ? 'green' : 'gray'}`}>
                    {card.isActive ? '사용중' : '비활성'}
                  </span>
                </td>
                <td>
                  <div style={{ display: 'flex', gap: 6 }}>
                    <button className="doc-btn" style={{ padding: '2px 8px', fontSize: 11 }}
                      onClick={() => openEdit(card)}>수정</button>
                    <button className="doc-btn" style={{ padding: '2px 8px', fontSize: 11 }}
                      onClick={() => toggleActive(card)}>
                      {card.isActive ? '비활성' : '활성'}
                    </button>
                  </div>
                </td>
              </tr>
            ))
          )}
        </tbody>
      </table>

      {/* 수령 이력 */}
      <div className="section-head" style={{ marginTop: 32 }}>
        <h2>카드 수령 이력</h2>
        <span className="meta">{assignments.length}건</span>
      </div>
      <table className="doc-table">
        <thead>
          <tr>
            <th>카드</th>
            <th>선생님</th>
            <th>수령일시</th>
            <th>반납일시</th>
            <th>비고</th>
          </tr>
        </thead>
        <tbody>
          {assignments.length === 0 ? (
            <tr><td colSpan={5} style={{ textAlign: 'center', padding: 24, color: 'var(--muted)' }}>
              수령 이력 없음
            </td></tr>
          ) : (
            assignments.map(a => (
              <tr key={a.id}>
                <td style={{ fontWeight: 600 }}>{a.card?.cardName || '-'}</td>
                <td>{a.teacherName}</td>
                <td style={{ fontSize: 12 }}>{new Date(a.takenAt).toLocaleString('ko-KR')}</td>
                <td style={{ fontSize: 12 }}>
                  {a.returnedAt ? new Date(a.returnedAt).toLocaleString('ko-KR') : (
                    <span className="doc-tag green">사용중</span>
                  )}
                </td>
                <td style={{ fontSize: 12, color: 'var(--muted)' }}>{a.note || '-'}</td>
              </tr>
            ))
          )}
        </tbody>
      </table>

      {/* 등록/수정 모달 */}
      {showAddModal && (
        <div style={{
          position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.3)',
          display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 50,
        }} onClick={() => setShowAddModal(false)}>
          <div className="doc" style={{ width: 400, padding: '24px 28px' }}
            onClick={e => e.stopPropagation()}>
            <div className="doc-header">
              <h2 style={{ fontSize: 16, margin: 0 }}>
                {editCard ? '카드 수정' : '새 카드 등록'}
              </h2>
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              <div className="doc-field" style={{ display: 'flex' }}>
                <label>카드명</label>
                <input value={form.cardName}
                  onChange={e => setForm({...form, cardName: e.target.value})}
                  placeholder="e.g. 신한법인카드" style={{ flex: 1 }} />
              </div>
              <div className="doc-field" style={{ display: 'flex' }}>
                <label>카드번호</label>
                <input value={form.cardNumber}
                  onChange={e => setForm({...form, cardNumber: e.target.value})}
                  placeholder="1234-5678-9012-3456" style={{ flex: 1 }} />
              </div>
              <div className="doc-field" style={{ display: 'flex' }}>
                <label>명의자</label>
                <input value={form.cardHolder}
                  onChange={e => setForm({...form, cardHolder: e.target.value})}
                  placeholder="(선택)" style={{ flex: 1 }} />
              </div>
              <div className="doc-field" style={{ display: 'flex' }}>
                <label>발급은행</label>
                <input value={form.issuingBank}
                  onChange={e => setForm({...form, issuingBank: e.target.value})}
                  placeholder="(선택)" style={{ flex: 1 }} />
              </div>
            </div>
            <div style={{ display: 'flex', gap: 8, marginTop: 20, justifyContent: 'flex-end' }}>
              <button className="doc-btn ghost" onClick={() => setShowAddModal(false)}>취소</button>
              <button className="doc-btn primary" onClick={submitForm}>
                {editCard ? '수정' : '등록'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
