import { invoke } from "@tauri-apps/api/core";

export type AutomationKind = "shortcut" | "fill" | "table_fill";

/** 화면 요소를 찾는 방법입니다. native 검증과 같은 값을 씁니다. */
export type LocatorKind = "id" | "name" | "label" | "css";

export interface AutomationLocator {
  kind: LocatorKind;
  value: string;
}

export interface AutomationField {
  label: string;
  locator: AutomationLocator;
}

/** 학생 × 날짜 행렬처럼 표를 채우는 레시피의 표 정보입니다. */
export interface AutomationTable {
  locator: AutomationLocator;
  identityHeaders: string[];
  dateLabels: string[];
}

export interface AutomationRecipe {
  id: string;
  name: string;
  targetUrl: string;
  kind: AutomationKind;
  /** 자동입력 레시피가 채울 필드입니다. 바로가기는 값을 갖지 않습니다. */
  fields?: AutomationField[];
  /** 표 입력 레시피의 표 정보입니다. 그 밖의 종류는 값을 갖지 않습니다. */
  table?: AutomationTable;
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

/**
 * 렌더러는 URL을 직접 넘기지 않고 저장된 레시피 id만 지정합니다.
 * 실제로 열 주소는 native 쪽에서 레시피 파일을 다시 읽어 결정합니다.
 */
export async function openAutomationRecipe(recipeId: string): Promise<void> {
  return invoke<void>("open_automation_recipe", { recipeId });
}

export type AutomationBridgeInfo = {
  port: number;
  token: string;
  extensionId: string;
};

/**
 * 확장 연결에 필요한 값입니다. 브리지는 127.0.0.1에만 열리고 토큰이 있어야
 * 요청을 받습니다.
 */
export async function fetchAutomationBridgeInfo(): Promise<AutomationBridgeInfo> {
  return invoke<AutomationBridgeInfo>("automation_bridge_info");
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

export function automationKindLabel(kind: AutomationKind): string {
  if (kind === "fill") {
    return "자동입력";
  }
  return kind === "table_fill" ? "표 입력" : "바로가기";
}

/**
 * 목록 한 줄에 보여 줄 요약입니다. 자동입력 레시피는 몇 개 필드를
 * 채우는지 함께 알려 줍니다.
 */
export function describeAutomationRecipe(recipe: AutomationRecipe): string {
  const host = describeAutomationTarget(recipe.targetUrl);
  if (recipe.kind === "fill") {
    const fieldCount = recipe.fields?.length ?? 0;
    return `필드 ${fieldCount}개 · ${host}`;
  }
  if (recipe.kind === "table_fill") {
    const identityCount = recipe.table?.identityHeaders?.length ?? 0;
    const dateCount = recipe.table?.dateLabels?.length ?? 0;
    return `식별 ${identityCount}열 · 날짜 ${dateCount}개 · ${host}`;
  }
  return host;
}

/** 표·자동입력 레시피는 페이지를 열어 확장에서 실행합니다. */
export function automationRunsInExtension(kind: AutomationKind): boolean {
  return kind !== "shortcut";
}
