import { useEffect, useState } from "react";
import {
  AppShell,
  Button,
  EmptyState,
  OfflineState,
  Status,
  type NavigationItem,
  type StatusTone,
} from "@school-collect/ui";

type RouteId = "overview" | "collections" | "settings";
type ConnectionState = "idle" | "checking" | "healthy" | "error" | "offline";

type ApiHealth = {
  status: string;
  service: string;
};

const apiBaseUrl = (
  import.meta.env.VITE_API_BASE_URL ?? "http://127.0.0.1:3000"
).replace(/\/$/, "");

const navigation: NavigationItem[] = [
  { id: "overview", icon: "activity", label: "홈", group: "업무" },
  { id: "collections", icon: "briefcase", label: "자료수합", group: "업무" },
  { id: "privacy", icon: "lock", label: "개인정보", group: "업무", disabled: true },
  { id: "purchase", icon: "briefcase", label: "구매·출장", group: "업무", disabled: true },
  { id: "curriculum", icon: "school", label: "교육과정", group: "교육", disabled: true },
  { id: "timetable", icon: "school", label: "시간표", group: "교육", disabled: true },
  { id: "plans", icon: "school", label: "교육계획", group: "교육", disabled: true },
  { id: "documents", icon: "briefcase", label: "문서자동화", group: "도구", disabled: true },
  { id: "settings", icon: "settings", label: "설정", group: "도구" },
];

const routeMeta: Record<
  RouteId,
  { title: string; description: string; emptyTitle: string; emptyDescription: string }
> = {
  overview: {
    title: "오늘의 업무",
    description: "",
    emptyTitle: "표시할 업무가 없습니다",
    emptyDescription: "업무 데이터가 연결되면 이곳에서 확인할 수 있습니다.",
  },
  collections: {
    title: "자료수합",
    description: "",
    emptyTitle: "표시할 수합이 없습니다",
    emptyDescription: "수합 데이터 연결 후 이곳에 표시됩니다.",
  },
  settings: {
    title: "설정",
    description: "",
    emptyTitle: "",
    emptyDescription: "",
  },
};

const connectionCopy: Record<
  ConnectionState,
  { label: string; tone: StatusTone; description: string }
> = {
  idle: {
    label: "확인 전",
    tone: "neutral",
    description: "API 연결 상태를 확인할 수 있습니다.",
  },
  checking: {
    label: "확인 중",
    tone: "info",
    description: "API에 연결하고 있습니다.",
  },
  healthy: {
    label: "연결됨",
    tone: "success",
    description: "API가 정상적으로 응답했습니다.",
  },
  error: {
    label: "확인 필요",
    tone: "danger",
    description: "API에 연결하지 못했습니다. 연결 상태를 확인한 뒤 다시 시도하세요.",
  },
  offline: {
    label: "오프라인",
    tone: "warning",
    description: "네트워크가 연결되면 다시 확인할 수 있습니다.",
  },
};

function isApiHealth(value: unknown): value is ApiHealth {
  if (!value || typeof value !== "object") {
    return false;
  }

  const body = value as Record<string, unknown>;
  return typeof body.status === "string" && typeof body.service === "string";
}

function initialOnlineState() {
  return typeof navigator === "undefined" || navigator.onLine;
}

function routeFromHash(): RouteId {
  if (typeof window === "undefined") {
    return "overview";
  }

  const candidate = window.location.hash.replace(/^#\/?/, "");
  return Object.hasOwn(routeMeta, candidate) ? (candidate as RouteId) : "overview";
}

export default function App() {
  const [activeRoute, setActiveRoute] = useState<RouteId>(routeFromHash);
  const [connectionState, setConnectionState] =
    useState<ConnectionState>("idle");
  const [apiHealth, setApiHealth] = useState<ApiHealth | null>(null);
  const [isOnline, setIsOnline] = useState(initialOnlineState);

  useEffect(() => {
    function handleOnline() {
      setIsOnline(true);
      setConnectionState((current) =>
        current === "offline" ? "idle" : current,
      );
    }

    function handleOffline() {
      setIsOnline(false);
      setConnectionState("offline");
      setApiHealth(null);
    }

    window.addEventListener("online", handleOnline);
    window.addEventListener("offline", handleOffline);

    return () => {
      window.removeEventListener("online", handleOnline);
      window.removeEventListener("offline", handleOffline);
    };
  }, []);

  useEffect(() => {
    function handleHashChange() {
      setActiveRoute(routeFromHash());
    }

    if (!window.location.hash) {
      window.history.replaceState(null, "", "#/overview");
    }

    window.addEventListener("hashchange", handleHashChange);
    return () => window.removeEventListener("hashchange", handleHashChange);
  }, []);

  const currentRoute = routeMeta[activeRoute];
  const connection = connectionCopy[connectionState];

  async function checkApi() {
    if (!isOnline) {
      setConnectionState("offline");
      return;
    }

    const controller = new AbortController();
    const timeout = window.setTimeout(() => controller.abort(), 5000);
    setConnectionState("checking");
    setApiHealth(null);

    try {
      const response = await fetch(`${apiBaseUrl}/health`, {
        headers: { Accept: "application/json" },
        signal: controller.signal,
      });

      if (!response.ok) {
        throw new Error("health request failed");
      }

      const body: unknown = await response.json();
      if (!isApiHealth(body)) {
        throw new Error("invalid health response");
      }

      setApiHealth(body);
      setConnectionState("healthy");
    } catch {
      setApiHealth(null);
      setConnectionState("error");
    } finally {
      window.clearTimeout(timeout);
    }
  }

  return (
    <AppShell
      activeNavigationId={activeRoute}
      banner={
        !isOnline ? (
          <div className="app-offline-banner" role="status">
            <OfflineState
              className="app-offline-banner__state"
              description="네트워크 연결을 확인한 뒤 다시 시도하세요."
              title="오프라인 상태입니다"
            />
            <Button onClick={checkApi} size="small" variant="secondary">
              다시 확인
            </Button>
          </div>
        ) : null
      }
      description={currentRoute.description}
      eyebrow={activeRoute === "overview" ? "홈" : undefined}
      navigation={navigation}
      onNavigationChange={(id) => {
        if (id in routeMeta) {
          window.location.hash = `/${id}`;
        }
      }}
      title={currentRoute.title}
    >
      {activeRoute === "settings" ? (
        <ConnectionSettings
          apiHealth={apiHealth}
          connection={connection}
          connectionState={connectionState}
          onCheckApi={checkApi}
        />
      ) : activeRoute === "overview" ? (
        <Overview />
      ) : (
        <RouteEmptyState route={currentRoute} />
      )}
    </AppShell>
  );
}

function Overview() {
  return (
    <section className="app-page" aria-labelledby="priority-heading">
      <h2 id="priority-heading">우선 확인할 업무</h2>
      <EmptyState
        description="업무 데이터가 연결되면 이곳에서 확인할 수 있습니다."
        title="표시할 업무가 없습니다"
      />
    </section>
  );
}

function ConnectionSettings({
  apiHealth,
  connection,
  connectionState,
  onCheckApi,
}: {
  apiHealth: ApiHealth | null;
  connection: (typeof connectionCopy)[ConnectionState];
  connectionState: ConnectionState;
  onCheckApi: () => void;
}) {
  return (
    <div className="app-page">
      <div className="app-overview-grid">
        <section className="app-connection-card">
          <div className="app-card-heading">
            <div>
              <p className="app-card-eyebrow">API 연결</p>
              <h2>서비스 상태</h2>
            </div>
            <Status tone={connection.tone}>{connection.label}</Status>
          </div>
          <p className="app-card-description">{connection.description}</p>
          {apiHealth ? (
            <dl className="app-health-details">
              <div>
                <dt>서비스</dt>
                <dd>{apiHealth.service}</dd>
              </div>
              <div>
                <dt>상태</dt>
                <dd>{apiHealth.status}</dd>
              </div>
            </dl>
          ) : null}
          <Button
            leadingIcon="refresh"
            loading={connectionState === "checking"}
            onClick={onCheckApi}
            variant="primary"
          >
            연결 확인
          </Button>
        </section>
      </div>
    </div>
  );
}

function RouteEmptyState({
  route,
}: {
  route: (typeof routeMeta)[RouteId];
}) {
  return (
    <div className="app-page">
      <EmptyState
        description={route.emptyDescription}
        title={route.emptyTitle}
      />
    </div>
  );
}
