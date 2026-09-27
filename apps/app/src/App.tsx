import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import {
  AppShell,
  Button,
  ErrorState,
  LoadingState,
  OfflineState,
  PermissionState,
  type NavigationItem,
} from "@school-collect/ui";
import { ApiError, fetchSession, refreshAuthSession, type SessionInfo } from "./api";
import { canManage, messageOf, useOnline } from "./helpers";
import { AssignmentPage } from "./pages/AssignmentPage";
import { AssignmentsPage } from "./pages/AssignmentsPage";
import { CreateSchoolCard, SignInView } from "./pages/AuthViews";
import { clearAuthSession, loadAuthSession, saveAuthSession } from "./session";
import { clearLocalDrafts } from "./drafts";
import { AutomationPage } from "./pages/AutomationPage";
import { CollectDetailPage } from "./pages/CollectDetailPage";
import { CollectsPage } from "./pages/CollectsPage";
import { MembersPage } from "./pages/MembersPage";
import { OverviewPage } from "./pages/OverviewPage";
import { SettingsPage } from "./pages/SettingsPage";
import {
  navIdFor,
  navigate,
  parseHash,
  routeNeedsMembership,
  type AppRoute,
} from "./routing";

type NavPage = Extract<
  AppRoute,
  {
    page:
      | "overview"
      | "collects"
      | "assignments"
      | "members"
      | "automation"
      | "settings";
  }
>["page"];

const routeMeta: Record<
  AppRoute["page"],
  { title: string; description: string }
> = {
  overview: {
    title: "홈",
    description: "제출할 수합과 진행 중인 수합, 마감 임박을 한눈에 봅니다.",
  },
  collects: {
    title: "자료수합",
    description: "수합을 만들고 상태를 걸러 본 뒤 상세에서 배포하거나 마감합니다.",
  },
  collect: {
    title: "수합 상세",
    description: "항목과 제출 현황을 보고, 상태에 맞는 배포·마감을 합니다.",
  },
  assignments: {
    title: "내 제출",
    description: "배정된 수합과 내 제출 상태를 확인합니다.",
  },
  assignment: {
    title: "제출 작성",
    description: "항목을 작성하고 임시 저장한 뒤 제출합니다.",
  },
  members: {
    title: "구성원",
    description: "이 학교의 구성원과 역할을 봅니다.",
  },
  automation: {
    title: "업무 자동화",
    description: "자주 쓰는 업무 화면을 사용자 정의 버튼으로 등록합니다.",
  },
  settings: {
    title: "설정",
    description: "연결 상태와 계정을 확인합니다.",
  },
};

export default function App() {
  const online = useOnline();
  const [token, setToken] = useState<string | null>(null);
  const [session, setSession] = useState<SessionInfo | null>(null);
  const [activeTenantId, setActiveTenantId] = useState<string | null>(null);
  const [route, setRoute] = useState<AppRoute>(() => parseHash(window.location.hash));
  const [bootError, setBootError] = useState<string | null>(null);
  const [signInOpen, setSignInOpen] = useState(false);
  const [retentionNotice, setRetentionNotice] = useState<string | null>(null);

  const signOut = useCallback(() => {
    const signingOutUser = session?.user.id;
    setToken(null);
    setSession(null);
    setActiveTenantId(null);
    setBootError(null);
    setSignInOpen(false);
    void (async () => {
      // 초안 정리는 세션이 남아 있는 동안 해야 하므로 먼저 끝냅니다.
      if (signingOutUser) {
        try {
          await clearLocalDrafts(signingOutUser);
        } catch (error) {
          setRetentionNotice(
            `이 컴퓨터에 저장된 초안을 지우지 못했습니다: ${messageOf(error)}`,
          );
        }
      }
      // 저장된 세션을 지워 다음 실행에서 토큰이 남지 않게 합니다.
      try {
        await clearAuthSession();
        setRetentionNotice(null);
      } catch (error) {
        setRetentionNotice(`저장된 세션을 지우지 못했습니다: ${messageOf(error)}`);
      }
    })();
    // 로그인 없이 쓸 수 있는 화면에 머무르고, 멤버십이 필요한 화면에서만 홈으로 돌아갑니다.
    if (routeNeedsMembership(parseHash(window.location.hash))) {
      navigate({ page: "overview" });
    }
  }, [session?.user.id]);

  /**
   * Restores the session saved in the OS credential store.
   *
   * An expired access token is refreshed when a refresh token exists; if that
   * fails the stored session is dropped and the user signs in again.
   */
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const stored = await loadAuthSession();
      if (cancelled || !stored) {
        return;
      }
      const expiringSoon =
        stored.expiresAtMs !== null && stored.expiresAtMs - Date.now() < 60_000;
      if (expiringSoon && stored.refreshToken) {
        try {
          const refreshed = await refreshAuthSession(stored.refreshToken);
          await saveAuthSession(refreshed);
          if (!cancelled) {
            setToken(refreshed.accessToken);
          }
          return;
        } catch {
          await clearAuthSession();
          return;
        }
      }
      if (expiringSoon) {
        await clearAuthSession();
        return;
      }
      setToken(stored.accessToken);
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    function onHashChange() {
      setRoute(parseHash(window.location.hash));
    }
    if (!window.location.hash) {
      window.location.hash = "#overview";
    }
    window.addEventListener("hashchange", onHashChange);
    return () => window.removeEventListener("hashchange", onHashChange);
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

  const activeTenant = useMemo(
    () =>
      session?.memberships.find((item) => item.tenantId === activeTenantId) ??
      null,
    [activeTenantId, session],
  );

  /**
   * Accepting an invitation changes the memberships, so the shell reloads the
   * session and keeps the current school when it is still valid.
   */
  const refreshSession = useCallback(async () => {
    if (!token) {
      return;
    }
    try {
      const value = await fetchSession(token);
      setSession(value);
      setActiveTenantId((current) =>
        current && value.memberships.some((item) => item.tenantId === current)
          ? current
          : (value.memberships[0]?.tenantId ?? null),
      );
      setBootError(null);
    } catch (error) {
      setBootError(messageOf(error));
    }
  }, [token]);

  const navigation = useMemo<NavigationItem[]>(() => {
    const manage = canManage(activeTenant?.role ?? "");
    // 로그인 전에는 전체 메뉴를 보여주고, 로그인 후에는 역할에 맞게 좁힙니다.
    const showManageItems = activeTenant === null || manage;
    const items: NavigationItem[] = [
      { id: "overview", icon: "activity", label: "홈", group: "업무" },
    ];
    // Contributor navigation hides collect/member IA. Server authorization
    // remains the source of truth if a hash route is opened directly.
    if (showManageItems) {
      items.push({
        id: "collects",
        icon: "briefcase",
        label: "자료수합",
        group: "업무",
      });
    }
    items.push({
      id: "assignments",
      icon: "inbox",
      label: "내 제출",
      group: "업무",
    });
    if (showManageItems) {
      items.push({
        id: "members",
        icon: "school",
        label: "구성원",
        group: "업무",
      });
    }
    items.push({
      id: "automation",
      icon: "sliders",
      label: "업무 자동화",
      group: "도구",
    });
    items.push({
      id: "settings",
      icon: "settings",
      label: "설정",
      group: "도구",
    });
    return items;
  }, [activeTenant]);

  function openSignIn() {
    setSignInOpen(true);
  }

  function localContent(): ReactNode {
    switch (route.page) {
      case "automation":
        return <AutomationPage />;
      case "settings":
        return (
          <SettingsPage
            onMembershipChanged={() => void refreshSession()}
            onSignIn={openSignIn}
            session={session}
            tenant={activeTenant}
            token={token}
          />
        );
      default:
        // 로컬 라우트를 추가하면 routing.ts의 membershipRoutes와 이 분기를 함께 갱신합니다.
        return null;
    }
  }

  function routeContent(): ReactNode {
    if (!routeNeedsMembership(route)) {
      return localContent();
    }

    if (!token) {
      return (
        <div className="app-page">
          <PermissionState
            action={<Button onClick={openSignIn}>로그인</Button>}
            description="이 화면은 학교 구성원으로 로그인한 뒤에 쓸 수 있습니다. 로그인 없이 쓸 수 있는 화면은 왼쪽 메뉴에 있습니다."
            title="로그인이 필요합니다"
          />
        </div>
      );
    }

    if (bootError) {
      return (
        <div className="app-page">
          <ErrorState
            action={
              <Button
                onClick={() => {
                  signOut();
                  openSignIn();
                }}
                variant="secondary"
              >
                다시 로그인
              </Button>
            }
            description={bootError}
            title="로그인 정보를 확인하지 못했습니다"
          />
        </div>
      );
    }

    if (!session) {
      return (
        <div className="app-page">
          <LoadingState description="계정 정보를 불러오고 있습니다." title="확인 중" />
        </div>
      );
    }

    if (!activeTenant) {
      return (
        <div className="app-page app-page--narrow">
          <CreateSchoolCard
            onCreated={(membership) => {
              setSession({
                ...session,
                memberships: [...session.memberships, membership],
              });
              setActiveTenantId(membership.tenantId);
            }}
            token={token}
          />
        </div>
      );
    }

    return (
      <>
        {route.page === "overview" ? (
          <OverviewPage
            role={activeTenant.role}
            tenantId={activeTenant.tenantId}
            token={token}
          />
        ) : null}
        {route.page === "collects" ? (
          <CollectsPage
            role={activeTenant.role}
            tenantId={activeTenant.tenantId}
            token={token}
          />
        ) : null}
        {route.page === "collect" ? (
          <CollectDetailPage
            collectId={route.id}
            role={activeTenant.role}
            tenantId={activeTenant.tenantId}
            token={token}
          />
        ) : null}
        {route.page === "assignments" ? (
          <AssignmentsPage tenantId={activeTenant.tenantId} token={token} />
        ) : null}
        {route.page === "assignment" ? (
          <AssignmentPage
            collectId={route.id}
            role={activeTenant.role}
            tenantId={activeTenant.tenantId}
            token={token}
            userId={session?.user.id ?? ""}
          />
        ) : null}
        {route.page === "members" ? (
          <MembersPage
            role={activeTenant.role}
            tenantId={activeTenant.tenantId}
            token={token}
          />
        ) : null}
      </>
    );
  }

  if (signInOpen) {
    return (
      <SignInView
        onBack={() => setSignInOpen(false)}
        onSignedIn={(next) => {
          setSignInOpen(false);
          setToken(next.accessToken);
          // 세션은 OS 보안 저장소에만 저장합니다(비밀번호는 저장하지 않습니다).
          void (async () => {
            try {
              await saveAuthSession(next);
              setRetentionNotice(null);
            } catch (error) {
              // 이번 실행에서는 로그인 상태로 쓸 수 있지만, 다시 열면 로그인해야 합니다.
              setRetentionNotice(
                `세션을 저장하지 못했습니다: ${messageOf(error)} 이 창을 닫으면 다시 로그인해야 합니다.`,
              );
            }
          })();
        }}
      />
    );
  }

  const meta = routeMeta[route.page];

  return (
    <AppShell
      activeNavigationId={navIdFor(route)}
      banner={
        online && !retentionNotice ? null : (
          <>
            {online ? null : (
              <div className="app-offline-banner">
                <OfflineState
                  className="app-offline-banner__state"
                  description="연결이 복구되면 다시 불러올 수 있습니다. 지금은 서버에 저장하거나 제출할 수 없습니다."
                  title="오프라인입니다"
                />
              </div>
            )}
            {retentionNotice ? (
              <p className="app-retention-notice" role="alert">
                {retentionNotice}
              </p>
            ) : null}
          </>
        )
      }
      description={meta.description}
      eyebrow={activeTenant?.tenantName}
      navigation={navigation}
      onNavigationChange={(id) => navigate({ page: id as NavPage })}
      sidebarFooter={
        <div className="app-sidebar-footer">
          <p className="app-sidebar-footer__name">
            {session
              ? (session.user.displayName ?? session.user.subject)
              : token
                ? "계정 확인 중"
                : "로그인하지 않음"}
          </p>
          {token ? (
            <Button onClick={signOut} size="small" variant="quiet">
              로그아웃
            </Button>
          ) : (
            <Button onClick={openSignIn} size="small" variant="secondary">
              로그인
            </Button>
          )}
        </div>
      }
      title={meta.title}
    >
      {routeContent()}
    </AppShell>
  );
}
