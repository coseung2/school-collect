import { useCallback, useEffect, useMemo, useState } from "react";
import {
  AppShell,
  Button,
  ErrorState,
  LoadingState,
  OfflineState,
  type NavigationItem,
} from "@school-collect/ui";
import { ApiError, fetchSession, type SessionInfo } from "./api";
import { canManage, messageOf, useOnline } from "./helpers";
import { AssignmentPage } from "./pages/AssignmentPage";
import { AssignmentsPage } from "./pages/AssignmentsPage";
import { CreateSchoolView, SignInView, Standalone } from "./pages/AuthViews";
import { CollectDetailPage } from "./pages/CollectDetailPage";
import { CollectsPage } from "./pages/CollectsPage";
import { MembersPage } from "./pages/MembersPage";
import { OverviewPage } from "./pages/OverviewPage";
import { SettingsPage } from "./pages/SettingsPage";
import {
  navIdFor,
  navigate,
  parseHash,
  type AppRoute,
} from "./routing";

type NavPage = Extract<
  AppRoute,
  { page: "overview" | "collects" | "assignments" | "members" | "settings" }
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

  const signOut = useCallback(() => {
    setToken(null);
    setSession(null);
    setActiveTenantId(null);
    navigate({ page: "overview" });
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

  const navigation = useMemo<NavigationItem[]>(() => {
    const manage = canManage(activeTenant?.role ?? "");
    const items: NavigationItem[] = [
      { id: "overview", icon: "activity", label: "홈", group: "업무" },
    ];
    // Contributor navigation hides collect/member IA. Server authorization
    // remains the source of truth if a hash route is opened directly.
    if (manage) {
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
    if (manage) {
      items.push({
        id: "members",
        icon: "school",
        label: "구성원",
        group: "업무",
      });
    }
    items.push({
      id: "settings",
      icon: "settings",
      label: "설정",
      group: "도구",
    });
    return items;
  }, [activeTenant]);

  if (!token) {
    return <SignInView onSignedIn={setToken} />;
  }

  if (bootError) {
    return (
      <Standalone>
        <ErrorState
          action={
            <Button onClick={signOut} variant="secondary">
              다시 로그인
            </Button>
          }
          description={bootError}
          title="로그인 정보를 확인하지 못했습니다"
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

  const meta = routeMeta[route.page];

  return (
    <AppShell
      activeNavigationId={navIdFor(route)}
      banner={
        online ? null : (
          <div className="app-offline-banner">
            <OfflineState
              className="app-offline-banner__state"
              description="연결이 복구되면 다시 불러올 수 있습니다. 지금은 서버에 저장하거나 제출할 수 없습니다."
              title="오프라인입니다"
            />
          </div>
        )
      }
      description={meta.description}
      eyebrow={activeTenant.tenantName}
      navigation={navigation}
      onNavigationChange={(id) => navigate({ page: id as NavPage })}
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
      title={meta.title}
    >
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
        />
      ) : null}
      {route.page === "members" ? (
        <MembersPage
          role={activeTenant.role}
          tenantId={activeTenant.tenantId}
          token={token}
        />
      ) : null}
      {route.page === "settings" ? (
        <SettingsPage session={session} tenant={activeTenant} token={token} />
      ) : null}
    </AppShell>
  );
}
