import { invoke } from "@tauri-apps/api/core";

/** Draft kept locally until the server accepts it. */
export type LocalDraft = {
  tenantId: string;
  userId: string;
  collectId: string;
  payload: Record<string, unknown>;
  baseVersion: number;
  updatedAtMs: number;
};

/** Writes the draft locally. This never depends on the network. */
export async function saveLocalDraft(
  draft: Omit<LocalDraft, "updatedAtMs">,
): Promise<LocalDraft> {
  return invoke<LocalDraft>("save_local_draft", { draft });
}

export async function loadLocalDraft(
  tenantId: string,
  userId: string,
  collectId: string,
): Promise<LocalDraft | null> {
  try {
    return await invoke<LocalDraft | null>("load_local_draft", {
      tenantId,
      userId,
      collectId,
    });
  } catch {
    return null;
  }
}

export async function listLocalDrafts(
  tenantId: string,
  userId: string,
): Promise<LocalDraft[]> {
  try {
    return await invoke<LocalDraft[]>("list_local_drafts", { tenantId, userId });
  } catch {
    return [];
  }
}

/** Removes one draft after the server accepted it. */
export async function deleteLocalDraft(
  tenantId: string,
  userId: string,
  collectId: string,
): Promise<void> {
  try {
    await invoke("delete_local_draft", { tenantId, userId, collectId });
  } catch {
    // The draft stays; the next save overwrites it anyway.
  }
}

/**
 * Signing out clears that user's drafts from this computer. A failure means
 * drafts may remain, so it is reported to the caller instead of swallowed.
 */
export async function clearLocalDrafts(userId: string): Promise<void> {
  await invoke("clear_local_drafts", { userId });
}
