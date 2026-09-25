export type AppRoute =
  | { page: "overview" }
  | { page: "collects" }
  | { page: "collect"; id: string }
  | { page: "assignments" }
  | { page: "assignment"; id: string }
  | { page: "members" }
  | { page: "settings" };

/**
 * 라우트별로 학교 멤버십(서버 데이터)이 필요한지 여부입니다.
 * 로그인 없이 쓸 수 있는 화면은 false로 두고, App.tsx의 로컬 화면 분기와 함께 갱신합니다.
 */
const membershipRoutes: Record<AppRoute["page"], boolean> = {
  overview: true,
  collects: true,
  collect: true,
  assignments: true,
  assignment: true,
  members: true,
  settings: false,
};

export function parseHash(hash: string): AppRoute {
  const path = hash.replace(/^#/, "").replace(/^\/+|\/+$/g, "");
  if (!path || path === "overview") {
    return { page: "overview" };
  }
  if (path === "collects") {
    return { page: "collects" };
  }
  if (path === "assignments") {
    return { page: "assignments" };
  }
  if (path === "members") {
    return { page: "members" };
  }
  if (path === "settings") {
    return { page: "settings" };
  }

  const collect = /^collects\/([^/]+)$/.exec(path);
  if (collect) {
    return { page: "collect", id: decodeURIComponent(collect[1]) };
  }

  const assignment = /^assignments\/([^/]+)$/.exec(path);
  if (assignment) {
    return { page: "assignment", id: decodeURIComponent(assignment[1]) };
  }

  return { page: "overview" };
}

export function hashFor(route: AppRoute): string {
  switch (route.page) {
    case "overview":
      return "#overview";
    case "collects":
      return "#collects";
    case "collect":
      return `#collects/${encodeURIComponent(route.id)}`;
    case "assignments":
      return "#assignments";
    case "assignment":
      return `#assignments/${encodeURIComponent(route.id)}`;
    case "members":
      return "#members";
    case "settings":
      return "#settings";
  }
}

export function navigate(route: AppRoute): void {
  const next = hashFor(route);
  if (window.location.hash === next) {
    window.dispatchEvent(new HashChangeEvent("hashchange"));
    return;
  }
  window.location.hash = next;
}

export function navIdFor(route: AppRoute): string {
  if (route.page === "collect") {
    return "collects";
  }
  if (route.page === "assignment") {
    return "assignments";
  }
  return route.page;
}

export function routeNeedsMembership(route: AppRoute): boolean {
  return membershipRoutes[route.page];
}
