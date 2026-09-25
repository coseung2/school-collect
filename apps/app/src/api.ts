const apiBaseUrl = (
  import.meta.env.VITE_API_BASE_URL ?? "http://127.0.0.1:3000"
).replace(/\/$/, "");

const supabaseUrl = (import.meta.env.VITE_SUPABASE_URL ?? "").replace(/\/$/, "");
const supabaseAnonKey = import.meta.env.VITE_SUPABASE_ANON_KEY ?? "";

/// The client can only sign in when it was built with the public project
/// address and publishable key. Both values are public by design; no server
/// credential is ever compiled into this bundle.
export const identityConfigured = Boolean(supabaseUrl && supabaseAnonKey);

export type Membership = {
  tenantId: string;
  tenantName: string;
  role: string;
};

export type SessionInfo = {
  user: {
    id: string;
    issuer: string;
    subject: string;
    displayName: string | null;
  };
  memberships: Membership[];
};

export type Collect = {
  id: string;
  title: string;
  status: string;
  dueAt: string | null;
  version: number;
  updatedAt: string;
  submissionStatus: string | null;
  submissionVersion: number | null;
};

export type Submission = {
  status: string;
  version: number;
  payload: Record<string, unknown>;
  submittedAt: string | null;
  updatedAt: string;
};

export type CollectDetail = {
  id: string;
  title: string;
  description: string;
  status: string;
  dueAt: string | null;
  version: number;
  updatedAt: string;
  submission: Submission | null;
  items: CollectItem[];
  progress: CollectProgress;
};

export type CollectItem = {
  key: string;
  label: string;
  required: boolean;
  position: number;
};

export type CollectItemInput = {
  key: string;
  label: string;
  required: boolean;
};

export type CollectProgress = {
  assigned: number;
  submitted: number;
};

export type Member = {
  userId: string;
  displayName: string | null;
  role: string;
};

export type Assignment = {
  collectId: string;
  title: string;
  status: string;
  dueAt: string | null;
  assignmentStatus: string;
  submissionStatus: string | null;
  submissionVersion: number | null;
};

export type CollectStatusRow = {
  userId: string;
  displayName: string | null;
  role: string;
  assignmentStatus: string | null;
  submissionStatus: string | null;
  submittedAt: string | null;
};

export type CollectStatus = {
  assigned: number;
  submitted: number;
  rows: CollectStatusRow[];
};

export class ApiError extends Error {
  readonly status: number;
  readonly code: string;
  readonly currentVersion: number | null;

  constructor(
    status: number,
    code: string,
    message: string,
    currentVersion: number | null,
  ) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.code = code;
    this.currentVersion = currentVersion;
  }
}

type RequestOptions = {
  method?: string;
  token?: string;
  tenantId?: string;
  body?: unknown;
};

async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const headers: Record<string, string> = { Accept: "application/json" };
  if (options.token) {
    headers.Authorization = `Bearer ${options.token}`;
  }
  if (options.tenantId) {
    headers["x-tenant-id"] = options.tenantId;
  }
  if (options.body !== undefined) {
    headers["Content-Type"] = "application/json";
  }

  const response = await fetch(`${apiBaseUrl}${path}`, {
    method: options.method ?? "GET",
    headers,
    body: options.body === undefined ? undefined : JSON.stringify(options.body),
  });

  const text = await response.text();
  const parsed: unknown = text ? JSON.parse(text) : null;

  if (!response.ok) {
    const body = (parsed ?? {}) as Record<string, unknown>;
    throw new ApiError(
      response.status,
      typeof body.code === "string" ? body.code : "request_failed",
      typeof body.message === "string" ? body.message : "요청을 처리하지 못했습니다.",
      typeof body.currentVersion === "number" ? body.currentVersion : null,
    );
  }

  return parsed as T;
}

type TokenResponse = {
  access_token?: string;
  user?: { id?: string; email?: string };
  error?: string;
  error_description?: string;
  msg?: string;
};

async function identityRequest(path: string, body: unknown): Promise<TokenResponse> {
  const response = await fetch(`${supabaseUrl}/auth/v1/${path}`, {
    method: "POST",
    headers: {
      apikey: supabaseAnonKey,
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body),
  });
  const parsed = (await response.json()) as TokenResponse;
  if (!response.ok) {
    throw new ApiError(
      response.status,
      parsed.error ?? "identity_failed",
      parsed.error_description ?? parsed.msg ?? "로그인에 실패했습니다.",
      null,
    );
  }
  return parsed;
}

export async function signInWithPassword(
  email: string,
  password: string,
): Promise<string> {
  const result = await identityRequest(
    "token?grant_type=password",
    { email, password },
  );
  if (!result.access_token) {
    throw new ApiError(401, "no_token", "로그인 응답에 토큰이 없습니다.", null);
  }
  return result.access_token;
}

export async function signUpWithPassword(
  email: string,
  password: string,
): Promise<boolean> {
  const result = await identityRequest("signup", { email, password });
  // The project requires a confirmed address before the first sign-in.
  return !result.access_token;
}

export const fetchSession = (token: string) =>
  request<SessionInfo>("/v1/session", { token });

export const createTenant = (token: string, name: string) =>
  request<Membership>("/v1/tenants", { method: "POST", token, body: { name } });

export const listCollects = (token: string, tenantId: string) =>
  request<{ collects: Collect[] }>("/v1/collects", { token, tenantId });

export const createCollect = (
  token: string,
  tenantId: string,
  title: string,
  description: string,
  items: CollectItemInput[] = [],
  assigneeUserIds: string[] = [],
  dueAt: string | null = null,
) =>
  request<Collect>("/v1/collects", {
    method: "POST",
    token,
    tenantId,
    body: { title, description, items, assigneeUserIds, dueAt },
  });

export const fetchCollect = (token: string, tenantId: string, id: string) =>
  request<CollectDetail>(`/v1/collects/${id}`, { token, tenantId });

export const fetchCollectStatus = (
  token: string,
  tenantId: string,
  id: string,
) => request<CollectStatus>(`/v1/collects/${id}/status`, { token, tenantId });

export const listMembers = (token: string, tenantId: string) =>
  request<{ members: Member[] }>("/v1/members", { token, tenantId });

/// Collects this caller has to submit. Managers see every collect in the
/// tenant through `listCollects`; a contributor starts from here.
export const listAssignments = (token: string, tenantId: string) =>
  request<{ assignments: Assignment[] }>("/v1/assignments", {
    token,
    tenantId,
  });

export const publishCollect = (token: string, tenantId: string, id: string) =>
  request<Collect>(`/v1/collects/${id}/publish`, {
    method: "POST",
    token,
    tenantId,
  });

export const closeCollect = (token: string, tenantId: string, id: string) =>
  request<Collect>(`/v1/collects/${id}/close`, {
    method: "POST",
    token,
    tenantId,
  });

export const saveDraft = (
  token: string,
  tenantId: string,
  id: string,
  expectedVersion: number,
  payload: Record<string, unknown>,
) =>
  request<Submission>(`/v1/collects/${id}/submission`, {
    method: "PUT",
    token,
    tenantId,
    body: { expectedVersion, payload },
  });

export const sendSubmission = (token: string, tenantId: string, id: string) =>
  request<Submission>(`/v1/collects/${id}/submission/submit`, {
    method: "POST",
    token,
    tenantId,
  });
