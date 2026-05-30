'use client'

import { useState, useEffect, useRef } from 'react'
import { useRouter } from 'next/navigation'
import type { TripDistanceStd } from '@/lib/types'
import { fmtMoney } from '@/lib/types'

// 장량초 기준 주요 도시 거리 (기본값 — API에 없을 때 fallback)
const DEFAULT_DISTANCES: Record<string, number> = {
  '경주': 35, '포항시내': 15, '영천': 60, '대구': 130,
  '부산': 130, '울산': 100, '창원': 140, '마산': 145,
  '서울': 370, '대전': 240, '광주': 280, '안동': 120,
  '구미': 140, '김천': 180, '전주': 300, '강릉': 300,
}

export default function NewTripPage() {
  const router = useRouter()
  const [saving, setSaving] = useState(false)
  const [distances, setDistances] = useState<TripDistanceStd[]>([])
  const [cameraActive, setCameraActive] = useState(false)
  const [capturedImage, setCapturedImage] = useState<string | null>(null)
  const videoRef = useRef<HTMLVideoElement>(null)
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const [stream, setStream] = useState<MediaStream | null>(null)

  const [form, setForm] = useState({
    teacherName: '',
    department: '',
    destination: '',
    purpose: '',
    transport: 'car' as const,
    tripDate: new Date().toISOString().slice(0, 10),
    fuelUnitPrice: 0,
    fuelEfficiency: 10.5,
    tollCost: 0,
    parkingCost: 0,
    otherCost: 0,
    note: '',
  })

  useEffect(() => {
    fetch('/api/trips/distances')
      .then(r => r.json())
      .then(setDistances)
      .catch(() => {})
  }, [])

  // 선택한 목적지에 따라 거리 자동 설정
  const distance = form.destination
    ? (distances.find(d => d.region === form.destination)?.baseDistance
      || DEFAULT_DISTANCES[form.destination] || 0)
    : 0

  const totalDistance = distance * (form.transport === 'car' ? 2 : 1) // 왕복

  // 유류대 = (거리 / 연비) × 유류단가
  const fuelCost = form.transport === 'car' && form.fuelUnitPrice > 0
    ? Math.round((totalDistance / form.fuelEfficiency) * form.fuelUnitPrice)
    : 0

  const totalCost = fuelCost + form.tollCost + form.parkingCost + form.otherCost

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
      alert('카메라를 켤 수 없습니다')
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

  // ─── 제출 ───
  const handleSubmit = async () => {
    if (!form.teacherName || !form.destination || !form.purpose || !form.tripDate) {
      alert('작성자, 목적지, 사유, 출장일은 필수입니다')
      return
    }

    setSaving(true)
    const res = await fetch('/api/trips/requests', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        teacherName: form.teacherName,
        department: form.department,
        destination: form.destination,
        purpose: form.purpose,
        transport: form.transport,
        tripDate: form.tripDate,
        distance: totalDistance,
        fuelUnitPrice: form.fuelUnitPrice,
        fuelEfficiency: form.fuelEfficiency,
        fuelCost,
        tollCost: form.tollCost,
        parkingCost: form.parkingCost,
        otherCost: form.otherCost,
        totalCost,
        receiptImageUrl: capturedImage || '',
        note: form.note,
      }),
    })

    setSaving(false)
    if (res.ok) {
      router.push('/trips')
    } else {
      const err = await res.json()
      alert('오류: ' + (err.error || '알 수 없는 오류'))
    }
  }

  // 지역 선택 옵션
  const regionOptions = [
    ...distances.map(d => d.region),
    ...Object.keys(DEFAULT_DISTANCES).filter(r => !distances.some(d => d.region === r)),
  ]
  const uniqueRegions = [...new Set(regionOptions)]

  return (
    <div className="doc" style={{ maxWidth: 700, margin: '0 auto', padding: '24px 28px' }}>
      <a href="/trips" className="doc-link" style={{ fontSize: 13, display: 'inline-block', marginBottom: 12 }}>
        ← 목록으로
      </a>

      <div className="doc-header">
        <h1>🚗 출장여비 신청서</h1>
        <div className="sub">유류대 · 톨비 자동 계산</div>
      </div>

      {/* 기본 정보 */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
          <div className="doc-field" style={{ display: 'flex' }}>
            <label>작성자</label>
            <input value={form.teacherName}
              onChange={e => setForm({...form, teacherName: e.target.value})}
              placeholder="이름" style={{ flex: 1 }} />
          </div>
          <div className="doc-field" style={{ display: 'flex' }}>
            <label>부서</label>
            <input value={form.department}
              onChange={e => setForm({...form, department: e.target.value})}
              placeholder="(선택)" style={{ flex: 1 }} />
          </div>
        </div>

        <div className="doc-field" style={{ display: 'flex' }}>
          <label>출장일</label>
          <input type="date" value={form.tripDate}
            onChange={e => setForm({...form, tripDate: e.target.value})}
            style={{ flex: 1 }} />
        </div>

        <div className="doc-field" style={{ display: 'flex' }}>
          <label>목적지</label>
          <input value={form.destination}
            onChange={e => setForm({...form, destination: e.target.value})}
            placeholder="지역명 입력 (e.g. 대구, 서울, 부산)"
            list="region-list" style={{ flex: 1 }} />
          <datalist id="region-list">
            {uniqueRegions.map(r => (
              <option key={r} value={r} />
            ))}
          </datalist>
        </div>

        <div className="doc-field" style={{ display: 'flex' }}>
          <label>출장 사유</label>
          <input value={form.purpose}
            onChange={e => setForm({...form, purpose: e.target.value})}
            placeholder="e.g. 정보화기기 교육 참석" style={{ flex: 1 }} />
        </div>

        <div className="doc-field" style={{ display: 'flex' }}>
          <label>교통수단</label>
          <select value={form.transport}
            onChange={e => setForm({...form, transport: e.target.value as any})}
            style={{ flex: 1 }}>
            <option value="car">🚗 자가용</option>
            <option value="bus">🚌 버스</option>
            <option value="train">🚄 기차</option>
            <option value="etc">기타</option>
          </select>
        </div>
      </div>

      {/* 거리/유류 계산 */}
      <div style={{ marginTop: 24 }}>
        <div className="section-head">
          <h2>📏 거리 및 유류비 계산</h2>
        </div>

        <div style={{ background: 'var(--subhead)', padding: 16, border: '1px solid var(--line)' }}>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 12 }}>
            <div>
              <div style={{ fontSize: 11, color: 'var(--muted)', fontWeight: 600, marginBottom: 4 }}>편도 거리</div>
              <div style={{ fontSize: 20, fontWeight: 800 }}>{distance} km</div>
            </div>
            <div>
              <div style={{ fontSize: 11, color: 'var(--muted)', fontWeight: 600, marginBottom: 4 }}>왕복 거리</div>
              <div style={{ fontSize: 20, fontWeight: 800 }}>{totalDistance} km</div>
            </div>
            <div>
              <div style={{ fontSize: 11, color: 'var(--muted)', fontWeight: 600, marginBottom: 4 }}>예상 유류대</div>
              <div style={{ fontSize: 20, fontWeight: 800, color: 'var(--blue)' }}>
                {form.transport === 'car' ? fmtMoney(fuelCost) : '-'}
              </div>
            </div>
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12, marginTop: 12 }}>
            <div className="doc-field" style={{ display: 'flex' }}>
              <label>유류단가</label>
              <input type="number" value={form.fuelUnitPrice || ''}
                onChange={e => setForm({...form, fuelUnitPrice: parseInt(e.target.value) || 0})}
                placeholder="원/L (공란시 미계산)" style={{ flex: 1 }} />
            </div>
            <div className="doc-field" style={{ display: 'flex' }}>
              <label>연비</label>
              <input type="number" step="0.1" value={form.fuelEfficiency}
                onChange={e => setForm({...form, fuelEfficiency: parseFloat(e.target.value) || 10.5})}
                style={{ flex: 1 }} />
            </div>
          </div>
        </div>
      </div>

      {/* 추가 비용 */}
      <div style={{ marginTop: 24 }}>
        <div className="section-head">
          <h2>💰 추가 비용</h2>
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 12 }}>
          <div className="doc-field" style={{ display: 'flex' }}>
            <label>톨비</label>
            <input type="number" value={form.tollCost || ''}
              onChange={e => setForm({...form, tollCost: parseInt(e.target.value) || 0})}
              placeholder="0" style={{ flex: 1 }} />
          </div>
          <div className="doc-field" style={{ display: 'flex' }}>
            <label>주차비</label>
            <input type="number" value={form.parkingCost || ''}
              onChange={e => setForm({...form, parkingCost: parseInt(e.target.value) || 0})}
              placeholder="0" style={{ flex: 1 }} />
          </div>
          <div className="doc-field" style={{ display: 'flex' }}>
            <label>기타</label>
            <input type="number" value={form.otherCost || ''}
              onChange={e => setForm({...form, otherCost: parseInt(e.target.value) || 0})}
              placeholder="0" style={{ flex: 1 }} />
          </div>
        </div>
      </div>

      {/* 합계 */}
      <div style={{
        marginTop: 24, padding: '16px 20px',
        border: '2px solid var(--ink)',
        display: 'flex', justifyContent: 'space-between', alignItems: 'center',
      }}>
        <div>
          <div style={{ fontSize: 12, color: 'var(--muted)', fontWeight: 600 }}>총 청구 금액</div>
          <div style={{ fontSize: 11, color: 'var(--muted)', marginTop: 2 }}>
            유류 {fmtMoney(fuelCost)} + 톨비 {fmtMoney(form.tollCost)} + 주차 {fmtMoney(form.parkingCost)} + 기타 {fmtMoney(form.otherCost)}
          </div>
        </div>
        <div style={{ fontSize: 28, fontWeight: 800, color: 'var(--blue)' }}>{fmtMoney(totalCost)}</div>
      </div>

      {/* 메모 */}
      <div className="doc-field" style={{ display: 'flex', marginTop: 12 }}>
        <label>비고</label>
        <input value={form.note}
          onChange={e => setForm({...form, note: e.target.value})}
          placeholder="참고사항 (선택)" style={{ flex: 1 }} />
      </div>

      {/* 영수증 첨부 */}
      <div style={{ marginTop: 16, border: '1px solid var(--line)', padding: 12 }}>
        <div style={{ fontSize: 12, fontWeight: 700, marginBottom: 8, color: 'var(--muted)' }}>
          🧾 영수증 첨부 (톨비/주차비 영수증)
        </div>
        {capturedImage ? (
          <div>
            <img src={capturedImage} alt="receipt"
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
          <button className="doc-btn" onClick={startCamera}>📸 카메라로 촬영</button>
        )}
      </div>

      {/* 제출 버튼 */}
      <div style={{ display: 'flex', gap: 8, marginTop: 24, justifyContent: 'flex-end' }}>
        <a href="/trips" className="doc-btn ghost" style={{ textDecoration: 'none' }}>취소</a>
        <button className="doc-btn primary" onClick={handleSubmit} disabled={saving}
          style={{ padding: '8px 24px' }}>
          {saving ? '저장중...' : '임시 저장'}
        </button>
      </div>
    </div>
  )
}
