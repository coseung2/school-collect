import { invoke } from "@tauri-apps/api/core";

export type AutomationKind = "shortcut";

export interface AutomationRecipe {
  id: string;
  name: string;
  targetUrl: string;
  kind: AutomationKind;
}

export async function listAutomationRecipes(): Promise<AutomationRecipe[]> {
  return invoke<AutomationRecipe[]>("list_automation_recipes");
}

export async function saveAutomationRecipe(
  recipe: AutomationRecipe,
): Promise<AutomationRecipe[]> {
  return invoke<AutomationRecipe[]>("save_automation_recipe", { recipe });
}

export async function deleteAutomationRecipe(
  recipeId: string,
): Promise<AutomationRecipe[]> {
  return invoke<AutomationRecipe[]>("delete_automation_recipe", { recipeId });
}

export async function openAutomationTarget(targetUrl: string): Promise<void> {
  return invoke<void>("open_automation_target", { targetUrl });
}

const UNKNOWN_AUTOMATION_ERROR = "알 수 없는 오류가 발생했습니다.";

/**
 * Tauri 명령이 거부되면 Rust에서 만든 한국어 문구가 문자열로 전달됩니다.
 * 그 밖의 내부 오류(TypeError 등)는 사용자 화면에 그대로 노출하지 않습니다.
 */
export function automationErrorMessage(error: unknown): string {
  if (typeof error === "string" && error.trim() !== "") {
    return error.trim();
  }
  return UNKNOWN_AUTOMATION_ERROR;
}

export function createShortcutRecipe(
  name: string,
  targetUrl: string,
): AutomationRecipe {
  return {
    id: crypto.randomUUID(),
    kind: "shortcut",
    name: name.trim(),
    targetUrl: targetUrl.trim(),
  };
}

export function describeAutomationTarget(targetUrl: string): string {
  try {
    const url = new URL(targetUrl);
    return url.host;
  } catch {
    return targetUrl;
  }
}
