import { type ReactNode, useEffect, useState } from "react";
import {
  ErrorState,
  OfflineState,
  PermissionState,
  type StatusTone,
} from "@school-collect/ui";
import { ApiError } from "./api";

const DUE_SOON_MS = 7 * 24 * 60 * 60 * 1000;
const URGENT_MS = 2 * 24 * 60 * 60 * 1000;

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
  assigned: "배정됨",
  started: "작성 중",
};

const roleLabel: Record<string, string> = {
  admin: "관리자",
  coordinator: "담당자",
  contributor: "작성자",
  viewer: "열람",
};

export function useOnline(): boolean {
  const [online, setOnline] = useState(
    typeof navigator === "undefined" ? true : navigator.onLine,
  );

  useEffect(() => {
    function sync() {
      setOnline(navigator.onLine);
    }
    window.addEventListener("online", sync);
    window.addEventListener("offline", sync);
    return () => {
      window.removeEventListener("online", sync);
      window.removeEventListener("offline", sync);
    };
  }, []);

  return online;
}

/**
 * Navigation and form gates are client convenience only. The server
 * authorizes every request; a hidden route is not an access grant, and a
 * visible control is not proof of permission.
 */
export function canManage(role: string): boolean {
  return role === "admin" || role === "coordinator";
}

export function canSubmit(role: string): boolean {
  return role === "admin" || role === "coordinator" || role === "contributor";
}

export function toneOf(value: string | null | undefined): StatusTone {
  if (!value) {
    return "neutral";
  }
  return statusTone[value] ?? "neutral";
}

export function labelOf(value: string | null | undefined): string {
  if (!value) {
    return "-";
  }
  return statusLabel[value] ?? value;
}

export function roleOf(value: string): string {
  return roleLabel[value] ?? value;
}

export function messageOf(error: unknown): string {
  if (error instanceof ApiError || error instanceof Error) {
    return error.message;
  }
  return "알 수 없는 오류가 발생했습니다.";
}

export function isForbidden(error: unknown): boolean {
  return (
    error instanceof ApiError &&
    (error.status === 403 || error.code === "forbidden")
  );
}

export function isOfflineError(error: unknown, online: boolean): boolean {
  if (!online) {
    return true;
  }
  return error instanceof TypeError;
}

export function formatDue(value: string | null | undefined): string | undefined {
  if (!value) {
    return undefined;
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return undefined;
  }
  return `마감 ${date.toLocaleString("ko-KR")}`;
}

export function dueTimestamp(value: string | null | undefined): number | null {
  if (!value) {
    return null;
  }
  const time = new Date(value).getTime();
  return Number.isNaN(time) ? null : time;
}

export function isUrgentDue(
  dueAt: string | null | undefined,
  status?: string | null,
): boolean {
  if (status === "closed") {
    return false;
  }
  const time = dueTimestamp(dueAt);
  if (time === null) {
    return false;
  }
  return time - Date.now() <= URGENT_MS;
}

export function isDueSoon(
  dueAt: string | null | undefined,
  status?: string | null,
): boolean {
  if (status === "closed") {
    return false;
  }
  const time = dueTimestamp(dueAt);
  if (time === null) {
    return false;
  }
  return time - Date.now() <= DUE_SOON_MS;
}

export function compareDue(
  left: string | null | undefined,
  right: string | null | undefined,
): number {
  const a = dueTimestamp(left);
  const b = dueTimestamp(right);
  if (a === null && b === null) {
    return 0;
  }
  if (a === null) {
    return 1;
  }
  if (b === null) {
    return -1;
  }
  return a - b;
}

export function payloadText(
  payload: Record<string, unknown> | null | undefined,
  key: string,
): string {
  const value = payload?.[key];
  if (typeof value === "string") {
    return value;
  }
  if (value == null) {
    return "";
  }
  return String(value);
}

export function itemKeyFromLabel(label: string, used: Set<string>): string {
  const slug = label
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
  const base = /^[a-z]/.test(slug) ? slug : `item_${slug || "field"}`;
  let key = base;
  let index = 2;
  while (used.has(key)) {
    key = `${base}_${index}`;
    index += 1;
  }
  used.add(key);
  return key;
}

export function RequestFailure({
  action,
  description,
  error,
  online,
  title,
}: {
  action?: ReactNode;
  description?: string;
  error: unknown;
  online: boolean;
  title: string;
}) {
  if (isOfflineError(error, online)) {
    return (
      <OfflineState
        action={action}
        description="네트워크에 다시 연결되면 이 화면을 불러올 수 있습니다."
        title="오프라인입니다"
      />
    );
  }
  if (isForbidden(error)) {
    return (
      <PermissionState
        action={action}
        description="이 화면의 최종 권한은 서버가 판단합니다. 접근이 거절되었습니다."
        title="권한이 없습니다"
      />
    );
  }
  return (
    <ErrorState
      action={action}
      description={description ?? messageOf(error)}
      title={title}
    />
  );
}
