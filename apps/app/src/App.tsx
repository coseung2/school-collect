import { useState } from "react";

type ApiHealth = {
  status: string;
  service: string;
};

const apiBaseUrl =
  import.meta.env.VITE_API_BASE_URL ?? "http://127.0.0.1:3000";

export default function App() {
  const [health, setHealth] = useState<string>("확인 전");

  async function checkApi() {
    setHealth("확인 중…");

    try {
      const response = await fetch(`${apiBaseUrl}/health`);
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      const body = (await response.json()) as ApiHealth;
      setHealth(`${body.service}: ${body.status}`);
    } catch (error) {
      const message = error instanceof Error ? error.message : "unknown error";
      setHealth(`연결 실패: ${message}`);
    }
  }

  return (
    <main className="foundation">
      <p className="eyebrow">SCHOOL COLLECT V2</p>
      <h1>Desktop foundation</h1>
      <p className="description">
        Tauri 2 + React/Vite 클라이언트와 Rust API의 실행 경계를 검증하는
        아키텍처 단계입니다. 제품 UI는 승인된 Figma 계약을 Stage 5에서
        구현합니다.
      </p>

      <section className="diagnostics" aria-labelledby="diagnostics-title">
        <div>
          <p className="label">API</p>
          <p id="diagnostics-title" className="value">
            {health}
          </p>
        </div>
        <button type="button" onClick={checkApi}>
          연결 확인
        </button>
      </section>
    </main>
  );
}
