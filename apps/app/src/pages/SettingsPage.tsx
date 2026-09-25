import { useState } from "react";
import { Button, Card, Status } from "@school-collect/ui";
import { type Membership, type SessionInfo } from "../api";
import { roleOf } from "../helpers";

export function SettingsPage({
  onSignIn,
  session,
  tenant,
  token,
}: {
  onSignIn: () => void;
  session: SessionInfo | null;
  tenant: Membership | null;
  token: string | null;
}) {
  const [health, setHealth] = useState<{ status: string; service: string } | null>(
    null,
  );
  const [error, setError] = useState<string | null>(null);

  async function check() {
    setError(null);
    try {
      const response = await fetch(
        `${(import.meta.env.VITE_API_BASE_URL ?? "http://127.0.0.1:3000").replace(/\/$/, "")}/ready`,
      );
      const body = (await response.json()) as { status: string; service: string };
      setHealth(body);
    } catch {
      setHealth(null);
      setError("API에 연결하지 못했습니다.");
    }
  }

  return (
    <div className="app-page">
      <Card className="app-connection-card">
        <div className="app-card-heading">
          <div>
            <p className="app-card-eyebrow">연결</p>
            <h2>서버 상태</h2>
          </div>
          <Status tone={health ? "success" : error ? "danger" : "neutral"}>
            {health ? "정상" : error ? "확인 필요" : "확인 전"}
          </Status>
        </div>
        <p className="app-card-description">
          서버가 준비 상태인지 확인합니다. 로그인 없이도 점검할 수 있습니다.
        </p>
        {error ? <p className="app-form__error">{error}</p> : null}
        {health ? (
          <p className="app-card-description">
            {health.service}: {health.status}
          </p>
        ) : null}
        <Button onClick={() => void check()} variant="secondary">
          연결 확인
        </Button>
      </Card>

      {session && tenant && token ? (
        <Card>
          <h2>계정</h2>
          <dl className="app-meta">
            <div>
              <dt>사용자</dt>
              <dd>{session.user.displayName ?? session.user.subject}</dd>
            </div>
            <div>
              <dt>학교</dt>
              <dd>{tenant.tenantName}</dd>
            </div>
            <div>
              <dt>권한</dt>
              <dd>{roleOf(tenant.role)}</dd>
            </div>
          </dl>
          <p className="app-card-description">
            접근 토큰은 이 창의 메모리에만 보관하며 디스크에 저장하지 않습니다.
          </p>
          <p className="app-card-description">세션 토큰 길이 {token.length}자</p>
        </Card>
      ) : (
        <Card>
          <h2>계정</h2>
          {token ? (
            <p className="app-card-description">계정 정보를 불러오는 중입니다.</p>
          ) : (
            <>
              <p className="app-card-description">
                로그인하면 학교와 권한 정보를 확인할 수 있습니다.
              </p>
              <Button onClick={onSignIn} variant="secondary">
                로그인
              </Button>
            </>
          )}
        </Card>
      )}
    </div>
  );
}
