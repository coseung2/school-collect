import { invoke } from "@tauri-apps/api/core";

import type { AuthSession } from "./api";

/** Session shape the native credential store keeps. */
type StoredSession = {
  accessToken: string;
  refreshToken?: string | null;
  userId: string;
  email?: string | null;
  expiresAtMs?: number | null;
};

function toStored(session: AuthSession): StoredSession {
  return {
    accessToken: session.accessToken,
    refreshToken: session.refreshToken,
    userId: session.userId,
    email: session.email,
    expiresAtMs: session.expiresAtMs,
  };
}

/** Stores the session in the OS credential store. */
export async function saveAuthSession(session: AuthSession): Promise<void> {
  await invoke("save_auth_session", { session: toStored(session) });
}

/**
 * Reads the stored session, or null when there is none (or the platform has no
 * credential store). A failure to read must not block signing in again.
 */
export async function loadAuthSession(): Promise<AuthSession | null> {
  try {
    const stored = await invoke<StoredSession | null>("load_auth_session");
    if (!stored?.accessToken) {
      return null;
    }
    return {
      accessToken: stored.accessToken,
      refreshToken: stored.refreshToken ?? null,
      userId: stored.userId,
      email: stored.email ?? null,
      expiresAtMs: stored.expiresAtMs ?? null,
    };
  } catch {
    return null;
  }
}

/** Removes the stored session. Signing out must not leave a token behind. */
export async function clearAuthSession(): Promise<void> {
  try {
    await invoke("clear_auth_session");
  } catch {
    // A missing entry is not an error; the in-memory session is cleared anyway.
  }
}
