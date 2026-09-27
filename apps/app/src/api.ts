import { invoke } from "@tauri-apps/api/core";

const apiBaseUrl = (
  import.meta.env.VITE_API_BASE_URL ?? "http://127.0.0.1:3000"
).replace(/\/$/, "");

const supabaseUrl = (import.meta.env.VITE_SUPABASE_URL ?? "").replace(/\/$/, "");
const supabaseAnonKey = import.meta.env.VITE_SUPABASE_ANON_KEY ?? "";

/**
 * Upstream provider the external-browser login asks the identity provider for.
 * The name is public; the redirect allowlist entry for the loopback callback
 * on the identity provider is what the owner still decides.
 */
const browserLoginProvider = import.meta.env.VITE_OIDC_PROVIDER ?? "google";

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

export type Invitation = {
  id: string;
  email: string;
  role: string;
  status: string;
  createdAt: string;
  expiresAt: string;
  acceptedAt: string | null;
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
  refresh_token?: string;
  expires_in?: number;
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
): Promise<AuthSession> {
  const result = await identityRequest(
    "token?grant_type=password",
    { email, password },
  );
  if (!result.access_token) {
    throw new ApiError(401, "no_token", "로그인 응답에 토큰이 없습니다.", null);
  }
  return {
    accessToken: result.access_token,
    refreshToken: result.refresh_token ?? null,
    userId: result.user?.id ?? "",
    email: result.user?.email ?? email,
    expiresAtMs: result.expires_in ? Date.now() + result.expires_in * 1000 : null,
  };
}

/** Session the native external-browser login hands back to the renderer. */
type BrowserLoginResult = {
  accessToken: string;
  refreshToken: string | null;
  expiresInSeconds: number | null;
  userId: string;
  email: string | null;
};

/** The native flow reports failures as a stable code plus a message. */
type BrowserLoginFailure = { code: string; message: string };

function isBrowserLoginFailure(caught: unknown): caught is BrowserLoginFailure {
  if (typeof caught !== "object" || caught === null) {
    return false;
  }
  const { code, message } = caught as { code?: unknown; message?: unknown };
  return typeof code === "string" && typeof message === "string";
}

/**
 * Signs in through the identity provider in the system browser (PKCE).
 *
 * Only the desktop shell can receive the loopback callback, so a plain browser
 * build gets a clear message instead of a silent failure. The renderer sends
 * public values only: the project address, the publishable anon key and the
 * provider name. No password is involved.
 */
export async function signInWithBrowser(): Promise<AuthSession> {
  let result: BrowserLoginResult;
  try {
    result = await invoke<BrowserLoginResult>("start_browser_login", {
      baseUrl: `${supabaseUrl}/auth/v1/`,
      anonKey: supabaseAnonKey,
      provider: browserLoginProvider,
    });
  } catch (caught) {
    if (isBrowserLoginFailure(caught)) {
      throw new ApiError(0, caught.code, caught.message, null);
    }
    throw new ApiError(
      0,
      "native_unavailable",
      "브라우저 로그인은 데스크톱 앱에서만 쓸 수 있습니다.",
      null,
    );
  }
  if (!result.accessToken) {
    throw new ApiError(401, "no_token", "로그인 응답에 토큰이 없습니다.", null);
  }
  return {
    accessToken: result.accessToken,
    refreshToken: result.refreshToken ?? null,
    userId: result.userId,
    email: result.email ?? null,
    expiresAtMs: result.expiresInSeconds
      ? Date.now() + result.expiresInSeconds * 1000
      : null,
  };
}

/** Stops an in-flight browser login. */
export async function cancelBrowserLogin(): Promise<void> {
  try {
    await invoke<boolean>("cancel_browser_login");
  } catch {
    // 이미 끝난 시도이거나 데스크톱 앱이 아닙니다. 시도는 시간 초과로도 끝납니다.
  }
}

/**
 * Session handed between the shell and the OS credential store.
 *
 * Only the identity provider's own tokens are kept; the password is never
 * stored.
 */
export type AuthSession = {
  accessToken: string;
  refreshToken: string | null;
  userId: string;
  email: string | null;
  expiresAtMs: number | null;
};

/** Exchanges a refresh token for a new access token. */
export async function refreshAuthSession(refreshToken: string): Promise<AuthSession> {
  const result = await identityRequest("token?grant_type=refresh_token", {
    refresh_token: refreshToken,
  });
  if (!result.access_token) {
    throw new ApiError(401, "no_token", "세션을 갱신하지 못했습니다.", null);
  }
  return {
    accessToken: result.access_token,
    refreshToken: result.refresh_token ?? refreshToken,
    userId: result.user?.id ?? "",
    email: result.user?.email ?? null,
    expiresAtMs: result.expires_in ? Date.now() + result.expires_in * 1000 : null,
  };
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

/**
 * Result export as CSV text.
 *
 * The response is not JSON, so it is fetched separately from `request`.
 */
export async function exportCollectCsv(
  token: string,
  tenantId: string,
  id: string,
): Promise<string> {
  const response = await fetch(`${apiBaseUrl}/v1/collects/${id}/export`, {
    headers: {
      Accept: "text/csv",
      Authorization: `Bearer ${token}`,
      "x-tenant-id": tenantId,
    },
  });
  const text = await response.text();
  if (!response.ok) {
    let code = "request_failed";
    let message = "결과를 내보내지 못했습니다.";
    try {
      const body = JSON.parse(text) as Record<string, unknown>;
      if (typeof body.code === "string") {
        code = body.code;
      }
      if (typeof body.message === "string") {
        message = body.message;
      }
    } catch {
      // The server sent something other than JSON; keep the default message.
    }
    throw new ApiError(response.status, code, message, null);
  }
  return text;
}

export const listMembers = (token: string, tenantId: string) =>
  request<{ members: Member[] }>("/v1/members", { token, tenantId });

/** Replaces the item list. A published collect may add items; answered items cannot be removed. */
export const updateCollectItems = (
  token: string,
  tenantId: string,
  id: string,
  expectedVersion: number,
  items: CollectItemInput[],
) =>
  request<CollectDetail>(`/v1/collects/${id}/items`, {
    method: "PUT",
    token,
    tenantId,
    body: { expectedVersion, items },
  });

/** Replaces who owes a submission. Targets that already saved work cannot be removed. */
export const updateCollectAssignments = (
  token: string,
  tenantId: string,
  id: string,
  expectedVersion: number,
  assigneeUserIds: string[],
) =>
  request<CollectDetail>(`/v1/collects/${id}/assignments`, {
    method: "PUT",
    token,
    tenantId,
    body: { expectedVersion, assigneeUserIds },
  });

export const listInvitations = (token: string, tenantId: string) =>
  request<{ invitations: Invitation[] }>("/v1/invitations", {
    token,
    tenantId,
  });

export const createInvitation = (
  token: string,
  tenantId: string,
  email: string,
  role: string,
) =>
  request<{ invitation: Invitation; code: string }>("/v1/invitations", {
    method: "POST",
    token,
    tenantId,
    body: { email, role },
  });

export const revokeInvitation = (
  token: string,
  tenantId: string,
  invitationId: string,
) =>
  request<Invitation>(`/v1/invitations/${invitationId}/revoke`, {
    method: "POST",
    token,
    tenantId,
  });

/// Accepting uses the caller's verified email, so no tenant header is needed:
/// the membership is created for the school recorded in the invitation.
export const acceptInvitation = (token: string, code: string) =>
  request<{ membership: Membership }>("/v1/invitations/accept", {
    method: "POST",
    token,
    body: { code },
  });

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

export type Attachment = {
  id: string;
  collectId: string;
  userId: string;
  itemKey: string;
  fileName: string;
  contentType: string;
  byteSize: number;
  status: "pending" | "uploading" | "stored" | "deleted";
  expiresAt: string;
  storedAt: string | null;
  createdAt: string;
  contentUrl: string;
};

/** Largest file the server accepts. */
export const MAX_ATTACHMENT_BYTES = 10 * 1024 * 1024;

// Browsers report HWP/HWPX inconsistently (often as an empty type), so the
// declared type is derived from the extension when the browser gives none.
const TYPE_BY_EXTENSION: Record<string, string> = {
  pdf: "application/pdf",
  hwp: "application/x-hwp",
  hwpx: "application/vnd.hancom.hwpx",
  docx: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
  xlsx: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
  pptx: "application/vnd.openxmlformats-officedocument.presentationml.presentation",
  xls: "application/vnd.ms-excel",
  ppt: "application/vnd.ms-powerpoint",
  doc: "application/msword",
  png: "image/png",
  jpg: "image/jpeg",
  jpeg: "image/jpeg",
  txt: "text/plain",
  zip: "application/zip",
};

/** File types the server accepts, for the file picker. */
export const ATTACHMENT_ACCEPT = Object.keys(TYPE_BY_EXTENSION)
  .map((extension) => `.${extension}`)
  .join(",");

export function attachmentContentType(file: File): string | null {
  const extension = file.name.split(".").pop()?.toLowerCase() ?? "";
  return TYPE_BY_EXTENSION[extension] ?? null;
}

/**
 * Files for one collect. `mine` returns only the caller's own files, which is
 * what the submission screen needs even for a manager who also answers;
 * `review` returns everything the caller may see (every file for a manager).
 */
export const listAttachments = (
  token: string,
  tenantId: string,
  collectId: string,
  scope: "mine" | "review",
) =>
  request<{ attachments: Attachment[] }>(
    `/v1/collects/${collectId}/attachments${scope === "mine" ? "?scope=mine" : ""}`,
    { token, tenantId },
  );

/**
 * Opens a slot, then sends the bytes. The server derives the object key and
 * checks size, type and checksum; the file name is only a label.
 */
export async function uploadAttachment(
  token: string,
  tenantId: string,
  collectId: string,
  itemKey: string,
  file: File,
): Promise<Attachment> {
  const contentType = attachmentContentType(file);
  if (!contentType) {
    throw new ApiError(400, "attachment_type_not_allowed", "올릴 수 없는 파일 형식입니다.", null);
  }
  if (file.size === 0) {
    throw new ApiError(400, "attachment_empty", "빈 파일은 올릴 수 없습니다.", null);
  }
  if (file.size > MAX_ATTACHMENT_BYTES) {
    throw new ApiError(400, "attachment_too_large", "파일은 10MB까지 올릴 수 있습니다.", null);
  }
  const slot = await request<{
    attachment: Attachment;
    upload: { kind: string; url: string; method: string };
  }>(`/v1/collects/${collectId}/attachments`, {
    method: "POST",
    token,
    tenantId,
    body: { itemKey, fileName: file.name, contentType, byteSize: file.size },
  });

  try {
    const response = await fetch(`${apiBaseUrl}${slot.upload.url}`, {
      method: slot.upload.method,
      headers: {
        Authorization: `Bearer ${token}`,
        "x-tenant-id": tenantId,
        "Content-Type": contentType,
      },
      body: file,
    });
    const text = await response.text();
    const parsed = (text ? JSON.parse(text) : null) as Record<string, unknown> | null;
    if (!response.ok) {
      throw new ApiError(
        response.status,
        typeof parsed?.code === "string" ? parsed.code : "request_failed",
        typeof parsed?.message === "string" ? parsed.message : "파일을 올리지 못했습니다.",
        null,
      );
    }
    return parsed as unknown as Attachment;
  } catch (error) {
    // The slot counts against the per-item limit; remove it so a failed upload
    // does not quietly use up a place. If this also fails (offline), the list
    // shows the unfinished slot with its own delete button.
    try {
      await deleteAttachment(token, tenantId, slot.attachment.id);
    } catch {
      // Left for the user to remove from the list.
    }
    throw error;
  }
}

export const deleteAttachment = (token: string, tenantId: string, attachmentId: string) =>
  request<null>(`/v1/attachments/${attachmentId}`, { method: "DELETE", token, tenantId });

/** Bytes of one attachment, for saving it to the downloads folder. */
export async function downloadAttachment(
  token: string,
  tenantId: string,
  attachment: Attachment,
): Promise<Uint8Array> {
  const response = await fetch(`${apiBaseUrl}${attachment.contentUrl}`, {
    headers: { Authorization: `Bearer ${token}`, "x-tenant-id": tenantId },
  });
  if (!response.ok) {
    let message = "파일을 받지 못했습니다.";
    try {
      const body = (await response.json()) as Record<string, unknown>;
      if (typeof body.message === "string") message = body.message;
    } catch {
      // Not JSON; keep the default message.
    }
    throw new ApiError(response.status, "download_failed", message, null);
  }
  return new Uint8Array(await response.arrayBuffer());
}
