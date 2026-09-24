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
