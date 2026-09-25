import { useState, type FormEvent, type ReactNode } from "react";
import { Button, Card, ErrorState, FormField } from "@school-collect/ui";
import {
  createTenant,
  identityConfigured,
  signInWithPassword,
  signUpWithPassword,
  type Membership,
} from "../api";
import { messageOf } from "../helpers";

export function Standalone({ children }: { children: ReactNode }) {
  return (
    <main className="app-standalone">
      <div className="app-standalone__panel">{children}</div>
    </main>
  );
}

export function SignInView({
  onBack,
  onSignedIn,
}: {
  onBack?: () => void;
  onSignedIn: (token: string) => void;
}) {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  async function submit(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const accessToken = await signInWithPassword(email.trim(), password);
      onSignedIn(accessToken);
    } catch (caught) {
      setError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  async function createAccount() {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const needsConfirmation = await signUpWithPassword(email.trim(), password);
      setNotice(
        needsConfirmation
          ? "가입 요청을 보냈습니다. 메일함에서 주소를 확인한 뒤 로그인하세요."
          : "계정이 만들어졌습니다. 이제 로그인할 수 있습니다.",
      );
    } catch (caught) {
      setError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Standalone>
      <Card className="app-signin">
        <div>
          <p className="app-card-eyebrow">SCHOOL COLLECT</p>
          <h1>로그인</h1>
          <p className="app-card-description">
            학교 계정으로 로그인하면 우리 학교의 수합 업무를 볼 수 있습니다.
          </p>
        </div>
        {!identityConfigured ? (
          <ErrorState
            description="빌드에 VITE_SUPABASE_URL 과 VITE_SUPABASE_ANON_KEY 가 없습니다."
            title="로그인 설정이 없습니다"
          />
        ) : (
          <form className="app-form" onSubmit={submit}>
            <FormField htmlFor="email" label="이메일" required>
              <input
                autoComplete="username"
                id="email"
                onChange={(event) => setEmail(event.target.value)}
                required
                type="email"
                value={email}
              />
            </FormField>
            <FormField htmlFor="password" label="비밀번호" required>
              <input
                autoComplete="current-password"
                id="password"
                minLength={6}
                onChange={(event) => setPassword(event.target.value)}
                required
                type="password"
                value={password}
              />
            </FormField>
            {error ? (
              <p className="app-form__error" role="alert">
                {error}
              </p>
            ) : null}
            {notice ? <p className="app-form__notice">{notice}</p> : null}
            <div className="app-form__actions">
              <Button loading={busy} type="submit">
                로그인
              </Button>
              <Button
                disabled={busy || !email || !password}
                onClick={createAccount}
                type="button"
                variant="secondary"
              >
                계정 만들기
              </Button>
            </div>
          </form>
        )}
        {onBack ? (
          <div className="app-form__actions">
            <Button onClick={onBack} type="button" variant="quiet">
              돌아가기
            </Button>
          </div>
        ) : null}
      </Card>
    </Standalone>
  );
}

export function CreateSchoolCard({
  onCreated,
  token,
}: {
  onCreated: (membership: Membership) => void;
  token: string;
}) {
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function submit(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      onCreated(await createTenant(token, name.trim()));
    } catch (caught) {
      setError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Card className="app-signin">
      <div>
        <p className="app-card-eyebrow">처음 설정</p>
        <h1>학교 등록</h1>
        <p className="app-card-description">
          아직 소속된 학교가 없습니다. 학교를 만들면 관리자 권한으로 시작합니다.
        </p>
      </div>
      <form className="app-form" onSubmit={submit}>
        <FormField htmlFor="school-name" label="학교 이름" required>
          <input
            id="school-name"
            onChange={(event) => setName(event.target.value)}
            required
            value={name}
          />
        </FormField>
        {error ? (
          <p className="app-form__error" role="alert">
            {error}
          </p>
        ) : null}
        <div className="app-form__actions">
          <Button loading={busy} type="submit">
            학교 만들기
          </Button>
        </div>
      </form>
    </Card>
  );
}
