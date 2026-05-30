'use client'

import { useState, useEffect, useRef } from 'react'
import type { CardExpenseCard, CardExpense, CardAssignment } from '@/lib/types'
import { expenseSourceLabel, fmtMoney } from '@/lib/types'

export default function CardsPage() {
  const [cards, setCards] = useState<CardExpenseCard[]>([])
  const [expenses, setExpenses] = useState<CardExpense[]>([])
  const [selectedCard, setSelectedCard] = useState<string>('all')
  const [loading, setLoading] = useState(true)
  const [showAssignModal, setShowAssignModal] = useState(false)
  const [showExpenseModal, setShowExpenseModal] = useState(false)
  const [showReceiptModal, setShowReceiptModal] = useState<string | null>(null)
  const [assignCard, setAssignCard] = useState<CardExpenseCard | null>(null)

  // 카메라 관련
  const videoRef = useRef<HTMLVideoElement>(null)
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const [cameraActive, setCameraActive] = useState(false)
  const [capturedImage, setCapturedImage] = useState<string | null>(null)
  const [stream, setStream] = useState<MediaStream | null>(null)

  // 지출 폼
  const [expForm, setExpForm] = useState({
    cardId: '',
    teacherName: '',
    amount: '',
    merchant: '',
    expenseDate: new Date().toISOString().slice(0, 16),
    memo: '',
  })

  // 수령 폼
  const [assignForm, setAssignForm] = useState({
    teacherName: '',
    note: '',
  })

  const loadData = async () => {
    setLoading(true)
    const [cardsRes, expensesRes] = await Promise.all([
      fetch('/api/cards?active=true'),
      fetch(`/api/cards/expenses?limit=200`),
    ])
    const cardsData = await cardsRes.json()
    const expensesData = await expensesRes.json()
    setCards(cardsData)
    setExpenses(expensesData)
    setLoading(false)
  }

  useEffect(() => { loadData() }, [])

  const filteredExpenses = selectedCard === 'all'
    ? expenses
    : expenses.filter(e => e.cardId === selectedCard)

  // ─── 카메라 ───
  const startCamera = async () => {
    try {
      const s = await navigator.mediaDevices.getUserMedia({ video: { facingMode: 'environment' } })
      setStream(s)
      setCameraActive(true)
      setTimeout(() => {
        if (videoRef.current) videoRef.current.srcObject = s
      }, 100)
    } catch (e) {
      alert('카메라를 켤 수 없습니다: ' + (e as Error).message)
    }
  }

  const capturePhoto = () => {
    if (!videoRef.current || !canvasRef.current) return
    const video = videoRef.current
    const canvas = canvasRef.current
    canvas.width = video.videoWidth
    canvas.height = video.videoHeight
    canvas.getContext('2d')!.drawImage(video, 0, 0)
    const dataUrl = canvas.toDataURL('image/jpeg', 0.8)
    setCapturedImage(dataUrl)
    stopCamera()
  }

  const stopCamera = () => {
    if (stream) {
      stream.getTracks().forEach(t => t.stop())
      setStream(null)
    }
    setCameraActive(false)
  }

  // ─── 지출 등록 ───
  const openExpenseModal = (cardId?: string) => {
    setExpForm({
      cardId: cardId || '',
      teacherName: '',
      amount: '',
      merchant: '',
      expenseDate: new Date().toISOString().slice(0, 16),
      memo: '',
    })
    setCapturedImage(null)
    setShowExpenseModal(true)
  }

  const submitExpense = async () => {
    if (!expForm.cardId || !expForm.amount || !expForm.merchant) {
      alert('카드, 금액, 사용처는 필수입니다')
      return
    }

    const res = await fetch('/api/cards/expenses', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        cardId: expForm.cardId,
        teacherName: expForm.teacherName,
        amount: parseInt(expForm.amount.replace(/,/g, '')),
        merchant: expForm.merchant,
        expenseDate: new Date(expForm.expenseDate).toISOString(),
        receiptImageUrl: capturedImage || '',
        memo: expForm.memo,
      }),
    })

    if (res.ok) {
      setShowExpenseModal(false)
      loadData()
    } else {
      const err = await res.json()
      alert('오류: ' + err.error)
    }
  }

  // ─── 카드 수령 ───
  const openAssignModal = (card: CardExpenseCard) => {
    setAssignCard(card)
    setAssignForm({ teacherName: '', note: '' })
    setShowAssignModal(true)
  }

  const submitAssign = async () => {
    if (!assignCard || !assignForm.teacherName) return
    const res = await fetch('/api/cards/assignments', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        cardId: assignCard.id,
        teacherName: assignForm.teacherName,
        note: assignForm.note,
      }),
    })

    if (res.ok) {
      setShowAssignModal(false)
      loadData()
    } else {
      const err = await res.json()
      alert(err.error || '등록 실패')
    }
  }

  const returnCard = async (assignmentId: string) => {
    await fetch('/api/cards/assignments', {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ id: assignmentId }),
    })
    loadData()
  }

  // ─── 영수증 보기 ───
  const viewReceipt = (url: string) => {
    if (url.startsWith('data:')) {
      window.open(url, '_blank')
    } else {
      setShowReceiptModal(url)
    }
  }

  // ─── 통계 ───
  const totalAmount = filteredExpenses.reduce((s, e) => s + e.amount, 0)
  const monthAmount = filteredExpenses
    .filter(e => {
      const d = new Date(e.expenseDate)
      const now = new Date()
      return d.getMonth() === now.getMonth() && d.getFullYear() === now.getFullYear()
    })
    .reduce((s, e) => s + e.amount, 0)

  if (loading) return <div className="doc-empty">로딩중...</div>

  return (
    <div className="doc" style={{ maxWidth: 1000, margin: '0 auto', padding: '24px 28px' }}>
      {/* 헤더 */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: 16 }}>
        <div className="doc-header" style={{ border: 'none', padding: 0, margin: 0 }}>
          <h1>💳 법인카드 지출 대장</h1>
          <div className="sub">카드 수령 · 지출 등록 · 영수증 관리</div>
        </div>
        <div style={{ display: 'flex', gap: 8 }}>
          <a href="/cards/admin" className="doc-btn" style={{ textDecoration: 'none' }}>⚙️ 카드 관리</a>
        </div>
      </div>

      {/* 통계 */}
      <div className="stat-grid">
        <div className="stat-box">
          <div className="num">{fmtMoney(monthAmount)}</div>
          <div className="label">이번달 지출</div>
        </div>
        <div className="stat-box">
          <div className="num">{fmtMoney(totalAmount)}</div>
          <div className="label">전체 지출</div>
        </div>
        <div className="stat-box">
          <div className="num">{filteredExpenses.length}건</div>
          <div className="label">총 지출 건수</div>
        </div>
        <div className="stat-box">
          <div className="num">{cards.filter(c => c.currentHolder).length}개</div>
          <div className="label">사용중인 카드</div>
        </div>
      </div>

      {/* 카드 현황 */}
      <div className="section-head">
        <h2>카드 현황</h2>
        <span className="meta">{cards.length}개</span>
      </div>
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 10, marginBottom: 20 }}>
        {cards.map(card => (
          <div key={card.id}
            style={{
              flex: '1 0 280px', padding: '14px 16px',
              border: card.currentHolder ? '2px solid var(--green)' : '1px solid var(--line)',
              background: card.currentHolder ? 'var(--green-weak)' : 'var(--white)',
            }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <div>
                <div style={{ fontWeight: 700, fontSize: 14 }}>{card.cardName}</div>
                <div style={{ fontSize: 12, color: 'var(--muted)', marginTop: 2 }}>{card.cardNumber}</div>
              </div>
              {card.currentHolder ? (
                <button className="doc-btn ghost" style={{ fontSize: 11, padding: '2px 8px' }}
                  onClick={() => {
                    if (confirm(`${card.currentHolder}님이 반납하시겠습니까?`)) {
                      // assignments에서 active 찾아서 반납
                      fetch('/api/cards/assignments?cardId=' + card.id + '&active=true')
                        .then(r => r.json())
                        .then((data: any[]) => {
                          if (data[0]) returnCard(data[0].id)
                        })
                    }
                  }}>
                  반납
                </button>
              ) : (
                <button className="doc-btn" style={{ fontSize: 11, padding: '2px 8px' }}
                  onClick={() => openAssignModal(card)}>
                  수령
                </button>
              )}
            </div>
            {card.currentHolder && (
              <div style={{ marginTop: 6, fontSize: 12, fontWeight: 600, color: 'var(--green)' }}>
                📋 {card.currentHolder} 님 사용중
              </div>
            )}
          </div>
        ))}
      </div>

      {/* 지출 내역 필터 */}
      <div className="toolbar">
        <select value={selectedCard} onChange={e => setSelectedCard(e.target.value)}
          style={{ border: '1px solid var(--line)', padding: '6px 10px', fontSize: 13 }}>
          <option value="all">모든 카드</option>
          {cards.map(c => (
            <option key={c.id} value={c.id}>{c.cardName}</option>
          ))}
        </select>
        <div className="toolbar-right">
          <button className="doc-btn primary" onClick={() => openExpenseModal()}>+ 지출 등록</button>
        </div>
      </div>

      {/* 지출 내역 테이블 */}
      <table className="doc-table">
        <thead>
          <tr>
            <th style={{ width: 120 }}>일시</th>
            <th>카드</th>
            <th>사용처</th>
            <th style={{ width: 80 }}>사용자</th>
            <th style={{ width: 100, textAlign: 'right' }}>금액</th>
            <th style={{ width: 60 }}>영수증</th>
            <th style={{ width: 40 }}>출처</th>
          </tr>
        </thead>
        <tbody>
          {filteredExpenses.length === 0 ? (
            <tr><td colSpan={7} style={{ textAlign: 'center', padding: 32, color: 'var(--muted)' }}>
              등록된 지출 내역이 없습니다
            </td></tr>
          ) : (
            filteredExpenses.map(exp => (
              <tr key={exp.id}>
                <td style={{ fontSize: 12, whiteSpace: 'nowrap' }}>
                  {new Date(exp.expenseDate).toLocaleString('ko-KR', {
                    month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit'
                  })}
                </td>
                <td style={{ fontSize: 12 }}>{exp.card?.cardName || '-'}</td>
                <td>
                  <div style={{ fontWeight: 600, fontSize: 13 }}>{exp.merchant}</div>
                  {exp.memo && <div style={{ fontSize: 11, color: 'var(--muted)' }}>{exp.memo}</div>}
                </td>
                <td style={{ fontSize: 12 }}>{exp.teacherName || '-'}</td>
                <td style={{ textAlign: 'right', fontWeight: 700, fontSize: 13 }}>{fmtMoney(exp.amount)}</td>
                <td>
                  {exp.receiptImageUrl ? (
                    <button className="doc-btn ghost" style={{ padding: '2px 8px', fontSize: 11 }}
                      onClick={() => viewReceipt(exp.receiptImageUrl)}>
                      🖼️ 보기
                    </button>
                  ) : (
                    <button className="doc-btn" style={{ padding: '2px 8px', fontSize: 11 }}
                      onClick={() => {
                        setExpForm({
                          cardId: exp.cardId,
                          teacherName: exp.teacherName,
                          amount: String(exp.amount),
                          merchant: exp.merchant,
                          expenseDate: exp.expenseDate.slice(0, 16),
                          memo: exp.memo,
                        })
                        setCapturedImage(null)
                        setShowExpenseModal(true)
                      }}>
                      📷 첨부
                    </button>
                  )}
                </td>
                <td style={{ fontSize: 11 }}>{expenseSourceLabel(exp.source)}</td>
              </tr>
            ))
          )}
        </tbody>
      </table>

      {/* ─── 지출 등록 모달 ─── */}
      {showExpenseModal && (
        <div style={{
          position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.3)',
          display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 50,
        }} onClick={() => { stopCamera(); setShowExpenseModal(false) }}>
          <div className="doc" style={{
            width: 460, maxHeight: '90vh', overflow: 'auto', padding: '24px 28px',
          }} onClick={e => e.stopPropagation()}>

            <div className="doc-header">
              <h2 style={{ fontSize: 16, margin: 0 }}>지출 등록</h2>
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              <div className="doc-field" style={{ display: 'flex' }}>
                <label>카드</label>
                <select value={expForm.cardId} onChange={e => setExpForm({...expForm, cardId: e.target.value})}
                  style={{ flex: 1 }}>
                  <option value="">선택</option>
                  {cards.map(c => (
                    <option key={c.id} value={c.id}>
                      {c.cardName} ({c.currentHolder || '보관중'})
                    </option>
                  ))}
                </select>
              </div>
              <div className="doc-field" style={{ display: 'flex' }}>
                <label>사용자</label>
                <input value={expForm.teacherName}
                  onChange={e => setExpForm({...expForm, teacherName: e.target.value})}
                  placeholder="자동매칭 또는 입력" style={{ flex: 1 }} />
              </div>
              <div className="doc-field" style={{ display: 'flex' }}>
                <label>사용처</label>
                <input value={expForm.merchant}
                  onChange={e => setExpForm({...expForm, merchant: e.target.value})}
                  placeholder="가맹점명" style={{ flex: 1 }} />
              </div>
              <div className="doc-field" style={{ display: 'flex' }}>
                <label>금액</label>
                <input value={expForm.amount}
                  onChange={e => setExpForm({...expForm, amount: e.target.value})}
                  placeholder="0" type="number" style={{ flex: 1 }} />
              </div>
              <div className="doc-field" style={{ display: 'flex' }}>
                <label>일시</label>
                <input type="datetime-local" value={expForm.expenseDate}
                  onChange={e => setExpForm({...expForm, expenseDate: e.target.value})}
                  style={{ flex: 1 }} />
              </div>
              <div className="doc-field" style={{ display: 'flex' }}>
                <label>메모</label>
                <input value={expForm.memo}
                  onChange={e => setExpForm({...expForm, memo: e.target.value})}
                  placeholder="적요 (선택)" style={{ flex: 1 }} />
              </div>
            </div>

            {/* 영수증 카메라 */}
            <div style={{ marginTop: 16, border: '1px solid var(--line)', padding: 12 }}>
              <div style={{ fontSize: 12, fontWeight: 700, marginBottom: 8, color: 'var(--muted)' }}>
                🧾 영수증 첨부
              </div>

              {capturedImage ? (
                <div>
                  <img src={capturedImage} alt="captured receipt"
                    style={{ maxWidth: '100%', maxHeight: 200, border: '1px solid var(--line)' }} />
                  <div style={{ display: 'flex', gap: 6, marginTop: 8 }}>
                    <button className="doc-btn" onClick={() => { setCapturedImage(null); startCamera() }}>
                      다시 찍기
                    </button>
                    <button className="doc-btn ghost" onClick={() => setCapturedImage(null)}>
                      삭제
                    </button>
                  </div>
                </div>
              ) : cameraActive ? (
                <div>
                  <video ref={videoRef} autoPlay playsInline
                    style={{ maxWidth: '100%', maxHeight: 200, background: '#000' }} />
                  <div style={{ display: 'flex', gap: 6, marginTop: 8 }}>
                    <button className="doc-btn primary" onClick={capturePhoto}>📸 촬영</button>
                    <button className="doc-btn ghost" onClick={stopCamera}>취소</button>
                  </div>
                  <canvas ref={canvasRef} style={{ display: 'none' }} />
                </div>
              ) : (
                <button className="doc-btn" onClick={startCamera}>📸 카메라 켜기</button>
              )}
            </div>

            <div style={{ display: 'flex', gap: 8, marginTop: 20, justifyContent: 'flex-end' }}>
              <button className="doc-btn ghost" onClick={() => { stopCamera(); setShowExpenseModal(false) }}>
                취소
              </button>
              <button className="doc-btn primary" onClick={submitExpense}>등록</button>
            </div>
          </div>
        </div>
      )}

      {/* ─── 카드 수령 모달 ─── */}
      {showAssignModal && assignCard && (
        <div style={{
          position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.3)',
          display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 50,
        }} onClick={() => setShowAssignModal(false)}>
          <div className="doc" style={{ width: 360, padding: '24px 28px' }}
            onClick={e => e.stopPropagation()}>

            <div className="doc-header">
              <h2 style={{ fontSize: 16, margin: 0 }}>카드 수령</h2>
              <div className="sub" style={{ marginTop: 4 }}>{assignCard.cardName} ({assignCard.cardNumber})</div>
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              <div className="doc-field" style={{ display: 'flex' }}>
                <label>선생님</label>
                <input value={assignForm.teacherName}
                  onChange={e => setAssignForm({...assignForm, teacherName: e.target.value})}
                  placeholder="이름" style={{ flex: 1 }} />
              </div>
              <div className="doc-field" style={{ display: 'flex' }}>
                <label>비고</label>
                <input value={assignForm.note}
                  onChange={e => setAssignForm({...assignForm, note: e.target.value})}
                  placeholder="용도 (선택)" style={{ flex: 1 }} />
              </div>
            </div>

            <div style={{ display: 'flex', gap: 8, marginTop: 20, justifyContent: 'flex-end' }}>
              <button className="doc-btn ghost" onClick={() => setShowAssignModal(false)}>취소</button>
              <button className="doc-btn primary" onClick={submitAssign}>수령 등록</button>
            </div>
          </div>
        </div>
      )}

      {/* 영수증 모달 */}
      {showReceiptModal && (
        <div style={{
          position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.5)',
          display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 50,
        }} onClick={() => setShowReceiptModal(null)}>
          <div style={{ maxWidth: '90vw', maxHeight: '90vh' }}
            onClick={e => e.stopPropagation()}>
            <img src={showReceiptModal} alt="receipt"
              style={{ maxWidth: '100%', maxHeight: '85vh' }} />
            <div style={{ textAlign: 'center', marginTop: 8 }}>
              <button className="doc-btn" onClick={() => setShowReceiptModal(null)}>닫기</button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
