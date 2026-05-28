'use client'

import { useState, useEffect } from 'react'
import type { PurchaseItem, PurchaseRequest } from '@/lib/types'

export default function PurchaseAdminPage() {
  const [items, setItems] = useState<PurchaseItem[]>([])
  const [requests, setRequests] = useState<PurchaseRequest[]>([])
  const [loading, setLoading] = useState(true)
  const [tab, setTab] = useState<'items' | 'requests'>('items')

  // 등록 폼
  const [form, setForm] = useState({
    name: '', spec: '', unitPrice: '', unit: 'EA', link: '',
    description: '', category: '기타',
  })
  const [saving, setSaving] = useState(false)
  const [formMsg, setFormMsg] = useState('')

  const fetchItems = async () => {
    const res = await fetch('/api/purchase/items')
    setItems(await res.json())
  }
  const fetchRequests = async () => {
    const res = await fetch('/api/purchase/requests')
    setRequests(await res.json())
  }

  useEffect(() => { fetchItems(); fetchRequests().then(() => setLoading(false)) }, [])

  // 금액 포맷
  const fmt = (n: number) => n.toLocaleString('ko-KR') + '원'

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!form.name) { setFormMsg('물품명은 필수입니다'); return }
    setSaving(true)
    setFormMsg('')

    const res = await fetch('/api/purchase/items', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        name: form.name,
        spec: form.spec,
        unitPrice: Number(form.unitPrice) || 0,
        unit: form.unit,
        link: form.link,
        description: form.description,
        category: form.category,
      }),
    })

    if (res.ok) {
      setFormMsg('✅ 등록 완료')
      setForm({ name: '', spec: '', unitPrice: '', unit: 'EA', link: '', description: '', category: '기타' })
      await fetchItems()
    } else {
      const data = await res.json()
      setFormMsg('❌ ' + (data.error || '오류'))
    }
    setSaving(false)
  }

  const downloadCSV = () => {
    // K-에듀파인 품목내역 업로드 양식: 순번 | 품명 | 규격 | 수량 | 단위 | 단가 | 금액 | 비고
    const rows: string[][] = []
    items.forEach((item, i) => {
      if (!item.totalQuantity) return
      const totalAmount = item.totalQuantity * item.unitPrice
      const detail = requests
        .filter(r => r.itemId === item.id)
        .map(r => `${r.grade}-${r.classNum}:${r.quantity}`)
        .join(', ')
      rows.push([
        String(i + 1),
        item.name,
        item.spec || '',
        String(item.totalQuantity),
        item.unit,
        String(item.unitPrice),
        String(totalAmount),
        detail,
      ])
    })

    const BOM = '\uFEFF'
    const csvContent = BOM + [
      ['순번', '품명', '규격', '수량', '단위', '단가', '금액', '비고(학급별내역)'],
      ...rows
    ].map(row => row.map(c => `"${c}"`).join(',')).join('\n')

    const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = '정보화기기_구입요청_품목내역.csv'
    a.click()
    URL.revokeObjectURL(url)
  }

  if (loading) return <div className="doc-empty">로딩중...</div>

  return (
    <div className="doc" style={{ maxWidth: 960, margin: '0 auto', padding: '24px 28px' }}>
      <div className="doc-header" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-end' }}>
        <div>
          <h1>구입요청 관리</h1>
          <div className="sub">물품 등록 / 신청 집계 / CSV 다운로드</div>
        </div>
        <div style={{ display: 'flex', gap: 6 }}>
          <button className={`doc-btn ${tab === 'items' ? 'primary' : ''}`}
            onClick={() => setTab('items')}>
            물품 등록
          </button>
          <button className={`doc-btn ${tab === 'requests' ? 'primary' : ''}`}
            onClick={() => setTab('requests')}>
            신청 집계
          </button>
        </div>
      </div>

      {/* 통계 */}
      <div className="stat-grid">
        <div className="stat-box">
          <div className="num">{items.length}</div>
          <div className="label">등록 물품</div>
        </div>
        <div className="stat-box">
          <div className="num">{requests.length}</div>
          <div className="label">신청 학급</div>
        </div>
        <div className="stat-box">
          <div className="num">{fmt(
            items.reduce((sum, i) => sum + (i.totalAmount || 0), 0)
          )}</div>
          <div className="label">총 예상금액</div>
        </div>
        <div className="stat-box">
          <div className="num">{items.reduce((sum, i) => sum + (i.totalQuantity || 0), 0)}</div>
          <div className="label">총 수량</div>
        </div>
      </div>

      {tab === 'items' ? (
        <div>
          {/* 등록 폼 */}
          <div className="section-head">
            <h2>물품 등록</h2>
          </div>
          <form onSubmit={handleCreate} style={{ marginBottom: 24 }}>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8, marginBottom: 8 }}>
              <div className="doc-field" style={{ flex: '1 1 200px' }}>
                <label>물품명 *</label>
                <input value={form.name} onChange={e => setForm({...form, name: e.target.value})}
                  placeholder="예: 삼성 모니터 27인치" required />
              </div>
              <div className="doc-field" style={{ flex: '1 1 120px' }}>
                <label>단가</label>
                <input type="number" value={form.unitPrice}
                  onChange={e => setForm({...form, unitPrice: e.target.value})}
                  placeholder="450000" />
              </div>
              <div className="doc-field" style={{ flex: '0 0 80px' }}>
                <label>단위</label>
                <select value={form.unit} onChange={e => setForm({...form, unit: e.target.value})}>
                  <option value="EA">EA</option>
                  <option value="SET">SET</option>
                  <option value="BOX">BOX</option>
                </select>
              </div>
              <div className="doc-field" style={{ flex: '1 1 140px' }}>
                <label>카테고리</label>
                <select value={form.category} onChange={e => setForm({...form, category: e.target.value})}>
                  <option value="컴퓨터">컴퓨터</option>
                  <option value="모니터">모니터</option>
                  <option value="태블릿">태블릿</option>
                  <option value="주변기기">주변기기</option>
                  <option value="소프트웨어">소프트웨어</option>
                  <option value="기타">기타</option>
                </select>
              </div>
            </div>
            <div style={{ display: 'flex', gap: 8, marginBottom: 8 }}>
              <div className="doc-field" style={{ flex: 1 }}>
                <label>규격</label>
                <input value={form.spec} onChange={e => setForm({...form, spec: e.target.value})}
                  placeholder="27인치 IPS, FHD" />
              </div>
              <div className="doc-field" style={{ flex: 1 }}>
                <label>링크</label>
                <input value={form.link} onChange={e => setForm({...form, link: e.target.value})}
                  placeholder="https://..." />
              </div>
            </div>
            <div style={{ display: 'flex', gap: 8 }}>
              <div className="doc-field" style={{ flex: 1 }}>
                <label>설명</label>
                <input value={form.description} onChange={e => setForm({...form, description: e.target.value})}
                  placeholder="설명 (선택)" />
              </div>
              <button type="submit" className="doc-btn primary" disabled={saving}
                style={{ minWidth: 100, height: 32 }}>
                {saving ? '등록중...' : '등록'}
              </button>
            </div>
            {formMsg && (
              <div style={{ marginTop: 8, fontSize: 13, fontWeight: 600,
                color: formMsg.startsWith('✅') ? 'var(--green)' : 'var(--red)' }}>
                {formMsg}
              </div>
            )}
          </form>

          {/* 등록된 물품 목록 */}
          <div className="section-head">
            <h2>등록 물품 ({items.length})</h2>
            {items.some(i => i.totalQuantity && i.totalQuantity > 0) && (
              <button className="doc-btn green" onClick={downloadCSV} style={{ fontSize: 12 }}>
                📥 CSV 다운로드
              </button>
            )}
          </div>
          {items.length === 0 ? (
            <div className="doc-empty">등록된 물품이 없습니다</div>
          ) : (
            <table className="doc-table">
              <thead>
                <tr>
                  <th>품명</th>
                  <th>규격</th>
                  <th style={{ textAlign: 'right' }}>단가</th>
                  <th style={{ textAlign: 'center' }}>신청학급</th>
                  <th style={{ textAlign: 'center' }}>총수량</th>
                  <th style={{ textAlign: 'right' }}>예상금액</th>
                </tr>
              </thead>
              <tbody>
                {items.map((item, i) => (
                  <tr key={item.id} style={i % 2 === 0 ? { background: 'var(--subhead)' } : {}}>
                    <td>
                      <div style={{ fontWeight: 600 }}>{item.name}</div>
                      <span className="doc-tag gray" style={{ marginTop: 2 }}>{item.category}</span>
                    </td>
                    <td>{item.spec || '-'}</td>
                    <td style={{ textAlign: 'right' }}>{fmt(item.unitPrice)}</td>
                    <td style={{ textAlign: 'center' }}>
                      {item.requestCount ? (
                        <span className="doc-tag blue">{item.requestCount}학급</span>
                      ) : (
                        <span style={{ color: 'var(--muted)' }}>0</span>
                      )}
                    </td>
                    <td style={{ textAlign: 'center' }}>{item.totalQuantity || 0} {item.unit}</td>
                    <td style={{ textAlign: 'right', fontWeight: 600 }}>
                      {item.totalAmount ? fmt(item.totalAmount) : '-'}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      ) : (
        <div>
          {/* 신청 상세 */}
          <div className="section-head">
            <h2>신청 내역 ({requests.length}건)</h2>
            {requests.length > 0 && (
              <button className="doc-btn green" onClick={downloadCSV} style={{ fontSize: 12 }}>
                📥 CSV 다운로드
              </button>
            )}
          </div>
          {requests.length === 0 ? (
            <div className="doc-empty">아직 신청 내역이 없습니다</div>
          ) : (
            <table className="doc-table">
              <thead>
                <tr>
                  <th>품명</th>
                  <th>학년반</th>
                  <th style={{ textAlign: 'center' }}>개수</th>
                  <th>작성자</th>
                </tr>
              </thead>
              <tbody>
                {(() => {
                  // 물품별 그룹핑
                  const grouped = new Map<string, { item: PurchaseItem; reqs: PurchaseRequest[] }>()
                  requests.forEach(req => {
                    const item = items.find(i => i.id === req.itemId)
                    if (!item || !item.name) return
                    if (!grouped.has(item.id)) {
                      grouped.set(item.id, { item, reqs: [] })
                    }
                    grouped.get(item.id)!.reqs.push(req)
                  })

                  const rows: React.ReactNode[] = []
                  grouped.forEach(({ item, reqs }) => {
                    reqs.forEach((req, j) => {
                      rows.push(
                        <tr key={req.id} style={j % 2 === 1 ? { background: 'var(--subhead)' } : {}}>
                          {j === 0 ? (
                            <td rowSpan={reqs.length} style={{ fontWeight: 600, verticalAlign: 'middle' }}>
                              {item.name}
                              <div style={{ fontSize: 11, color: 'var(--muted)' }}>{fmt(item.unitPrice)}/{item.unit}</div>
                            </td>
                          ) : null}
                          <td>{req.grade}-{req.classNum}반</td>
                          <td style={{ textAlign: 'center' }}>{req.quantity} {item.unit}</td>
                          <td>{req.submitter || '-'}</td>
                        </tr>
                      )
                    })
                  })
                  return rows
                })()}
              </tbody>
            </table>
          )}
        </div>
      )}
    </div>
  )
}