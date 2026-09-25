import { useCallback, useEffect, useState } from "react";
import {
  AppShell,
  Button,
  Card,
  EmptyState,
  ErrorState,
  FormField,
  ListRow,
  ListSurface,
  LoadingState,
  Status,
  type NavigationItem,
  type StatusTone,
} from "@school-collect/ui";
import {
  ApiError,
  closeCollect,
  createCollect,
  createTenant,
  fetchCollect,
  fetchSession,
  identityConfigured,
  listCollects,
  publishCollect,
  saveDraft,
  sendSubmission,
  signInWithPassword,
  signUpWithPassword,
  type Collect,
  type CollectDetail,
  type Membership,
  type SessionInfo,
} from "./api";
import {
  automationErrorMessage,
  createShortcutRecipe,
  deleteAutomationRecipe,
  describeAutomationTarget,
  listAutomationRecipes,
  openAutomationTarget,
  saveAutomationRecipe,
  type AutomationRecipe,
} from "./automation";

type RouteId = "overview" | "collects" | "automation" | "settings";

const navigation: NavigationItem[] = [
  { id: "overview", icon: "activity", label: "홈", group: "업무" },
  { id: "collects", icon: "briefcase", label: "자료수합", group: "업무" },
  { id: "automation", icon: "sliders", label: "업무 자동화", group: "도구" },
  { id: "settings", icon: "settings", label: "설정", group: "도구" },
];

const statusTone: Record<string, StatusTone> = {
  draft: "neutral",
  published: "info",
  closed: "success",
  submitted: "success",
  assigned: "neutral",
  started: "info",
};

const statusLabel: Record<string, string> = {
  draft: "작성 중",
  published: "진행 중",
  closed: "마감",
  submitted: "제출 완료",
};

function toneOf(value: string | null): StatusTone {
  if (!value) {
    return "neutral";
  }
  return statusTone[value] ?? "neutral";
}

function labelOf(value: string | null): string {
  if (!value) {
    return "-";
  }
  return statusLabel[value] ?? value;
}

function canManage(role: string): boolean {
  return role === "admin" || role === "coordinator";
}

function messageOf(error: unknown): string {
  return error instanceof ApiError ? error.message : "알 수 없는 오류가 발생했습니다.";
}

export default function App() {
  const [token, setToken] = useState<string | null>(null);
  const [session, setSession] = useState<SessionInfo | null>(null);
  const [activeTenantId, setActiveTenantId] = useState<string | null>(null);
  const [activeRoute, setActiveRoute] = useState<RouteId>("overview");
  const [bootError, setBootError] = useState<string | null>(null);

  const signOut = useCallback(() => {
    setToken(null);
    setSession(null);
    setActiveTenantId(null);
    setActiveRoute("overview");
  }, []);

  useEffect(() => {
    if (!token || session) {
      return;
    }
    let cancelled = false;
    fetchSession(token)
      .then((value) => {
        if (cancelled) return;
        setSession(value);
        setActiveTenantId(value.memberships[0]?.tenantId ?? null);
        setBootError(null);
      })
      .catch((error: unknown) => {
        if (cancelled) return;
        setBootError(messageOf(error));
        if (error instanceof ApiError && error.status === 401) {
          signOut();
        }
      });
    return () => {
      cancelled = true;
    };
  }, [session, signOut, token]);

  if (!token) {
    return <SignInView onSignedIn={setToken} />;
  }

  if (bootError) {
    return (
      <Standalone>
        <ErrorState
          description={bootError}
          title="로그인 정보를 확인하지 못했습니다"
          action={
            <Button onClick={signOut} variant="secondary">
              다시 로그인
            </Button>
          }
        />
      </Standalone>
    );
  }

  if (!session) {
    return (
      <Standalone>
        <LoadingState description="계정 정보를 불러오고 있습니다." title="확인 중" />
      </Standalone>
    );
  }

  const activeTenant =
    session.memberships.find((item) => item.tenantId === activeTenantId) ?? null;

  if (!activeTenant) {
    return (
      <CreateSchoolView
        onCreated={(membership) => {
          setSession({
            ...session,
            memberships: [...session.memberships, membership],
          });
          setActiveTenantId(membership.tenantId);
        }}
        token={token}
      />
    );
  }

  const routeMeta: Record<RouteId, { title: string; description: string }> = {
    overview: { title: "홈", description: "우리 학교의 수합 업무를 한눈에 봅니다." },
    collects: { title: "자료수합", description: "수합을 만들고, 작성하고, 마감합니다." },
    automation: {
      title: "업무 자동화",
      description: "자주 쓰는 업무 화면을 사용자 정의 버튼으로 등록합니다.",
    },
    settings: { title: "설정", description: "연결 상태와 계정을 확인합니다." },
  };

  return (
    <AppShell
      activeNavigationId={activeRoute}
      description={routeMeta[activeRoute].description}
      eyebrow={activeTenant.tenantName}
      navigation={navigation}
      onNavigationChange={(id) => setActiveRoute(id as RouteId)}
      sidebarFooter={
        <div className="app-sidebar-footer">
          <p className="app-sidebar-footer__name">
            {session.user.displayName ?? session.user.subject}
          </p>
          <Button onClick={signOut} size="small" variant="quiet">
            로그아웃
          </Button>
        </div>
      }
      title={routeMeta[activeRoute].title}
    >
      {activeRoute === "settings" ? (
        <SettingsView session={session} tenant={activeTenant} token={token} />
      ) : activeRoute === "automation" ? (
        <AutomationView />
      ) : (
        <CollectsView
          role={activeTenant.role}
          tenantId={activeTenant.tenantId}
          token={token}
          variant={activeRoute === "overview" ? "overview" : "full"}
        />
      )}
    </AppShell>
  );
}

function Standalone({ children }: { children: React.ReactNode }) {
  return (
    <main className="app-standalone">
      <div className="app-standalone__panel">{children}</div>
    </main>
  );
}

function SignInView({ onSignedIn }: { onSignedIn: (token: string) => void }) {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  async function submit(event: React.FormEvent) {
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
      </Card>
    </Standalone>
  );
}

function CreateSchoolView({
  token,
  onCreated,
}: {
  token: string;
  onCreated: (membership: Membership) => void;
}) {
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function submit(event: React.FormEvent) {
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
    <Standalone>
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
    </Standalone>
  );
}

function CollectsView({
  token,
  tenantId,
  role,
  variant,
}: {
  token: string;
  tenantId: string;
  role: string;
  variant: "overview" | "full";
}) {
  const [collects, setCollects] = useState<Collect[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [openId, setOpenId] = useState<string | null>(null);
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    try {
      const value = await listCollects(token, tenantId);
      setCollects(value.collects);
      setError(null);
    } catch (caught) {
      setError(messageOf(caught));
    }
  }, [tenantId, token]);

  useEffect(() => {
    void load();
  }, [load]);

  async function run(action: () => Promise<unknown>) {
    setBusy(true);
    try {
      await action();
      await load();
      setError(null);
    } catch (caught) {
      setError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  if (openId) {
    return (
      <CollectDetailView
        collectId={openId}
        onBack={() => setOpenId(null)}
        role={role}
        tenantId={tenantId}
        token={token}
      />
    );
  }

  const visible = collects ?? [];

  return (
    <div className="app-page">
      {error ? <ErrorState description={error} title="요청을 처리하지 못했습니다" /> : null}

      {canManage(role) && variant === "full" ? (
        <Card className="app-create">
          <h2>새 수합 만들기</h2>
          <form
            className="app-form"
            onSubmit={(event) => {
              event.preventDefault();
              void run(async () => {
                await createCollect(token, tenantId, title.trim(), description.trim());
                setTitle("");
                setDescription("");
              });
            }}
          >
            <FormField htmlFor="collect-title" label="제목" required>
              <input
                id="collect-title"
                onChange={(event) => setTitle(event.target.value)}
                required
                value={title}
              />
            </FormField>
            <FormField htmlFor="collect-description" label="설명">
              <textarea
                id="collect-description"
                onChange={(event) => setDescription(event.target.value)}
                rows={3}
                value={description}
              />
            </FormField>
            <div className="app-form__actions">
              <Button loading={busy} type="submit">
                초안 만들기
              </Button>
            </div>
          </form>
        </Card>
      ) : null}

      <section className="app-section">
        <div className="app-section-heading">
          <h2>{variant === "overview" ? "진행 중인 수합" : "수합 목록"}</h2>
          <Button onClick={() => void load()} size="small" variant="quiet">
            새로고침
          </Button>
        </div>
        {collects === null ? (
          <LoadingState description="수합 목록을 불러오고 있습니다." title="불러오는 중" />
        ) : visible.length === 0 ? (
          <EmptyState
            description={
              canManage(role)
                ? "위에서 첫 수합을 만들면 여기에 표시됩니다."
                : "담당자가 수합을 배포하면 여기에 표시됩니다."
            }
            title="아직 수합이 없습니다"
          />
        ) : (
          <ListSurface>
            {visible.map((collect) => (
              <ListRow
                action={
                  <div className="app-row-actions">
                    <Button
                      onClick={() => setOpenId(collect.id)}
                      size="small"
                      variant="secondary"
                    >
                      열기
                    </Button>
                    {canManage(role) && collect.status === "draft" ? (
                      <Button
                        disabled={busy}
                        onClick={() =>
                          void run(() => publishCollect(token, tenantId, collect.id))
                        }
                        size="small"
                      >
                        배포
                      </Button>
                    ) : null}
                    {canManage(role) && collect.status === "published" ? (
                      <Button
                        disabled={busy}
                        onClick={() =>
                          void run(() => closeCollect(token, tenantId, collect.id))
                        }
                        size="small"
                        variant="quiet"
                      >
                        마감
                      </Button>
                    ) : null}
                  </div>
                }
                description={
                  collect.dueAt
                    ? `마감 ${new Date(collect.dueAt).toLocaleDateString("ko-KR")}`
                    : undefined
                }
                meta={
                  collect.submissionStatus ? (
                    <Status tone={toneOf(collect.submissionStatus)}>
                      내 제출 {labelOf(collect.submissionStatus)}
                    </Status>
                  ) : undefined
                }
                status={
                  <Status tone={toneOf(collect.status)}>
                    {labelOf(collect.status)}
                  </Status>
                }
                title={collect.title}
              />
            ))}
          </ListSurface>
        )}
      </section>
    </div>
  );
}

function CollectDetailView({
  token,
  tenantId,
  role,
  collectId,
  onBack,
}: {
  token: string;
  tenantId: string;
  role: string;
  collectId: string;
  onBack: () => void;
}) {
  const [detail, setDetail] = useState<CollectDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState("");
  const [busy, setBusy] = useState(false);
  const [saved, setSaved] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const value = await fetchCollect(token, tenantId, collectId);
      setDetail(value);
      setNote(typeof value.submission?.payload?.note === "string" ? value.submission.payload.note : "");
      setError(null);
    } catch (caught) {
      setError(messageOf(caught));
    }
  }, [collectId, tenantId, token]);

  useEffect(() => {
    void load();
  }, [load]);

  if (error && !detail) {
    return (
      <div className="app-page">
        <ErrorState
          action={
            <Button onClick={onBack} variant="secondary">
              목록으로
            </Button>
          }
          description={error}
          title="수합을 열지 못했습니다"
        />
      </div>
    );
  }

  if (!detail) {
    return (
      <div className="app-page">
        <LoadingState description="수합을 불러오고 있습니다." title="불러오는 중" />
      </div>
    );
  }

  const submission = detail.submission;
  const editable = detail.status === "published" && submission?.status !== "submitted";

  async function persist(next: () => Promise<unknown>, message: string) {
    setBusy(true);
    setSaved(null);
    try {
      await next();
      await load();
      setError(null);
      setSaved(message);
    } catch (caught) {
      setError(messageOf(caught));
      if (caught instanceof ApiError && caught.code === "version_conflict") {
        await load();
      }
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="app-page">
      <div className="app-detail-head">
        <Button onClick={onBack} size="small" variant="quiet">
          ← 목록
        </Button>
        <Status tone={toneOf(detail.status)}>{labelOf(detail.status)}</Status>
      </div>

      <Card>
        <h2>{detail.title}</h2>
        {detail.description ? (
          <p className="app-card-description">{detail.description}</p>
        ) : null}
        <dl className="app-meta">
          <div>
            <dt>수합 버전</dt>
            <dd>{detail.version}</dd>
          </div>
          <div>
            <dt>내 제출</dt>
            <dd>{labelOf(submission?.status ?? null)}</dd>
          </div>
        </dl>
        {canManage(role) && detail.status === "published" ? (
          <Button
            disabled={busy}
            onClick={() =>
              void persist(
                () => closeCollect(token, tenantId, detail.id),
                "수합을 마감했습니다.",
              )
            }
            variant="secondary"
          >
            마감하기
          </Button>
        ) : null}
      </Card>

      <Card>
        <h2>내 제출 내용</h2>
        {error ? <ErrorState description={error} title="저장하지 못했습니다" /> : null}
        {saved ? <p className="app-form__notice">{saved}</p> : null}
        {!editable ? (
          <p className="app-card-description">
            {submission?.status === "submitted"
              ? "제출이 완료되어 더 이상 수정할 수 없습니다."
              : "수합이 진행 중일 때만 작성할 수 있습니다."}
          </p>
        ) : null}
        <FormField
          hint="서버가 버전을 확인하므로 다른 기기에서 동시에 저장하면 충돌로 알려줍니다."
          htmlFor="submission-note"
          label="내용"
        >
          <textarea
            disabled={!editable}
            id="submission-note"
            onChange={(event) => setNote(event.target.value)}
            rows={6}
            value={note}
          />
        </FormField>
        <div className="app-form__actions">
          <Button
            disabled={!editable || busy}
            onClick={() =>
              void persist(
                () =>
                  saveDraft(token, tenantId, detail.id, submission?.version ?? 0, {
                    note,
                  }),
                "초안을 저장했습니다.",
              )
            }
          >
            초안 저장
          </Button>
          <Button
            disabled={!editable || busy || !submission}
            onClick={() =>
              void persist(
                () => sendSubmission(token, tenantId, detail.id),
                "제출했습니다.",
              )
            }
            variant="secondary"
          >
            제출
          </Button>
        </div>
      </Card>
    </div>
  );
}

function AutomationView() {
  const [recipes, setRecipes] = useState<AutomationRecipe[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [name, setName] = useState("");
  const [targetUrl, setTargetUrl] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      setRecipes(await listAutomationRecipes());
      setLoadError(null);
    } catch (caught) {
      setRecipes(null);
      setLoadError(automationErrorMessage(caught));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function create(event: React.FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const next = await saveAutomationRecipe(createShortcutRecipe(name, targetUrl));
      setRecipes(next);
      setLoadError(null);
      setName("");
      setTargetUrl("");
      setNotice("업무 버튼을 등록했습니다.");
    } catch (caught) {
      setError(automationErrorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function remove(recipeId: string) {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      setRecipes(await deleteAutomationRecipe(recipeId));
      setLoadError(null);
      setNotice("업무 버튼을 삭제했습니다.");
    } catch (caught) {
      setError(automationErrorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function open(recipe: AutomationRecipe) {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      await openAutomationTarget(recipe.targetUrl);
    } catch (caught) {
      setError(automationErrorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="app-page">
      <Card className="app-create">
        <div>
          <p className="app-card-eyebrow">로컬 업무 버튼</p>
          <h2>바로가기 등록</h2>
          <p className="app-card-description">
            업무포털에 이미 로그인한 기본 브라우저에서 등록한 화면을 엽니다. 쿠키,
            인증서, 비밀번호는 School Collect에 저장하지 않습니다.
          </p>
        </div>

        <form className="app-form" onSubmit={create}>
          <FormField htmlFor="automation-name" label="버튼 이름" required>
            <input
              id="automation-name"
              maxLength={80}
              onChange={(event) => setName(event.target.value)}
              placeholder="예: 기안"
              required
              value={name}
            />
          </FormField>
          <FormField
            hint="http/https 주소만 저장합니다. 로그인 토큰이 포함된 URL은 등록하지 마세요."
            htmlFor="automation-target"
            label="대상 URL"
            required
          >
            <input
              id="automation-target"
              onChange={(event) => setTargetUrl(event.target.value)}
              placeholder="https://..."
              required
              type="url"
              value={targetUrl}
            />
          </FormField>

          {error ? (
            <p className="app-form__error" role="alert">
              {error}
            </p>
          ) : null}
          {notice ? <p className="app-form__notice">{notice}</p> : null}

          <div className="app-form__actions">
            <Button disabled={!name.trim() || !targetUrl.trim()} loading={busy} type="submit">
              버튼 추가
            </Button>
          </div>
        </form>
      </Card>

      <section className="app-section">
        <div className="app-section-heading">
          <div>
            <h2>내 업무 버튼</h2>
            <p className="app-card-description">
              다음 단계에서 현재 화면 등록, 표 자동입력, 신규 신청 감시를 같은 레시피에
              연결합니다.
            </p>
          </div>
          <Button disabled={loading} onClick={() => void load()} size="small" variant="quiet">
            새로고침
          </Button>
        </div>

        {loading ? (
          <LoadingState description="로컬 자동화 설정을 불러오고 있습니다." title="불러오는 중" />
        ) : loadError ? (
          <ErrorState
            action={
              <Button onClick={() => void load()} variant="secondary">
                다시 시도
              </Button>
            }
            description={loadError}
            title="업무 버튼을 불러오지 못했습니다"
          />
        ) : !recipes || recipes.length === 0 ? (
          <EmptyState
            description="위에서 기안, 품의, 출결처럼 자주 쓰는 화면을 첫 버튼으로 등록하세요."
            title="등록한 업무 버튼이 없습니다"
          />
        ) : (
          <ListSurface>
            {recipes.map((recipe) => (
              <ListRow
                action={
                  <div className="app-row-actions">
                    <Button
                      disabled={busy}
                      onClick={() => void open(recipe)}
                      size="small"
                      variant="secondary"
                    >
                      열기
                    </Button>
                    <Button
                      disabled={busy}
                      onClick={() => void remove(recipe.id)}
                      size="small"
                      variant="quiet"
                    >
                      삭제
                    </Button>
                  </div>
                }
                description={describeAutomationTarget(recipe.targetUrl)}
                key={recipe.id}
                status={<Status tone="info">바로가기</Status>}
                title={recipe.name}
              />
            ))}
          </ListSurface>
        )}
      </section>
    </div>
  );
}

function SettingsView({
  token,
  tenant,
  session,
}: {
  token: string;
  tenant: Membership;
  session: SessionInfo;
}) {
  const [health, setHealth] = useState<{ status: string; service: string } | null>(null);
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
          로그인 토큰이 유효한지 확인하고, 서버가 준비 상태인지 점검합니다.
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
            <dd>{tenant.role}</dd>
          </div>
        </dl>
        <p className="app-card-description">
          접근 토큰은 이 창의 메모리에만 보관하며 디스크에 저장하지 않습니다.
        </p>
        <p className="app-card-description">세션 토큰 길이 {token.length}자</p>
      </Card>
    </div>
  );
}
