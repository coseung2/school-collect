export type AppRoute =
  | { page: "overview" }
  | { page: "collects" }
  | { page: "collect"; id: string }
  | { page: "assignments" }
  | { page: "assignment"; id: string }
  | { page: "members" }
  | { page: "settings" };

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
