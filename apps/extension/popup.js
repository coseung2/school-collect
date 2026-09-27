import { automationPageCommand } from "./page-command.js";

const BRIDGE_URL = "http://127.0.0.1:43110";

const tokenInput = document.getElementById("token");
const nameInput = document.getElementById("name");
const statusEl = document.getElementById("status");
const pickerEl = document.getElementById("picker");
const pickedEl = document.getElementById("picked");
const candidatesEl = document.getElementById("candidates");
const fieldLabelInput = document.getElementById("field-label");
const fieldsEl = document.getElementById("fields");
const recipeNameInput = document.getElementById("recipe-name");
const recipeSelect = document.getElementById("recipe-select");
const previewResultsEl = document.getElementById("preview-results");
const valuesEl = document.getElementById("values");
const fillResultsEl = document.getElementById("fill-results");
const tablePickedEl = document.getElementById("table-picked");
const tableNameInput = document.getElementById("table-name");
const tableSelect = document.getElementById("table-select");
const tableValuesEl = document.getElementById("table-values");
const tableResultsEl = document.getElementById("table-results");
const watchPickedEl = document.getElementById("watch-picked");
const watchNameInput = document.getElementById("watch-name");
const watchSelect = document.getElementById("watch-select");
const watchResultsEl = document.getElementById("watch-results");
const suggestionsEl = document.getElementById("suggestions");

let picked = null;
let draftFields = [];
let fillRecipes = [];
let tableRecipes = [];
let watchRecipes = [];
let pickedTable = null;
let pickedWatch = null;
let valueInputs = [];

const SUGGESTION_KIND_TEXT = {
  fill: "자동입력",
  table_fill: "표 입력",
  watch: "감시",
};

const SUGGESTION_PATHS = {
  fill: "/v1/bridge/fill-recipes",
  table_fill: "/v1/bridge/table-recipes",
  watch: "/v1/bridge/watch-recipes",
};

function setStatus(message, tone) {
  statusEl.textContent = message;
  statusEl.dataset.tone = tone ?? "";
}

async function loadToken() {
  const stored = await chrome.storage.local.get("token");
  if (typeof stored.token === "string") {
    tokenInput.value = stored.token;
  }
}

async function bridgeFetch(path, options = {}) {
  const token = tokenInput.value.trim();
  if (!token) {
    throw new Error("연결 토큰을 먼저 저장하세요.");
  }
  let response;
  try {
    response = await fetch(`${BRIDGE_URL}${path}`, {
      ...options,
      headers: {
        ...(options.headers ?? {}),
        Authorization: `Bearer ${token}`,
      },
    });
  } catch {
    throw new Error("School Collect 앱이 실행 중인지 확인하세요.");
  }
  const text = await response.text();
  const body = text ? JSON.parse(text) : null;
  if (!response.ok) {
    throw new Error(body?.message ?? `요청이 실패했습니다 (${response.status})`);
  }
  return body;
}

async function activeTab() {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  return tab ?? null;
}

function normalizedPath(url) {
  const path = url.pathname.replace(/\/+$/, "");
  return path === "" ? "/" : path;
}

/**
 * Decides whether the active tab is the screen a recipe was recorded on.
 * Same origin alone is not enough: one work site hosts many screens, and a
 * recipe must never type into a different form on the same host.
 * - "match": same origin and path (query/fragment may differ, e.g. a month).
 * - "wrong-origin" / "wrong-screen": refuse.
 */
function screenMatch(recipe, tab) {
  if (!tab?.url || !/^https?:/i.test(tab.url)) {
    return { ok: false, reason: "not-web" };
  }
  const recipeUrl = new URL(recipe.targetUrl);
  const tabUrl = new URL(tab.url);
  if (recipeUrl.origin !== tabUrl.origin) {
    return { ok: false, reason: "wrong-origin", expected: recipeUrl.origin };
  }
  if (normalizedPath(recipeUrl) !== normalizedPath(tabUrl)) {
    return {
      ok: false,
      reason: "wrong-screen",
      expected: `${recipeUrl.origin}${normalizedPath(recipeUrl)}`,
    };
  }
  return { ok: true };
}

/** Returns true when the tab is the recipe's screen; otherwise shows why and returns false. */
function ensureRecipeScreen(recipe, tab, verb) {
  const match = screenMatch(recipe, tab);
  if (match.ok) {
    return true;
  }
  if (match.reason === "not-web") {
    setStatus(`http/https 화면에서만 ${verb} 수 있습니다.`, "error");
  } else if (match.reason === "wrong-origin") {
    setStatus(`이 레시피는 ${match.expected} 화면용입니다.`, "error");
  } else {
    setStatus(`같은 사이트의 다른 화면입니다. 이 레시피는 ${match.expected} 화면용입니다.`, "error");
  }
  return false;
}

async function runPageCommand(command, { allowFailure = false } = {}) {
  const tab = await activeTab();
  if (!tab?.id || !/^https?:/i.test(tab.url ?? "")) {
    throw new Error("http/https 화면에서만 사용할 수 있습니다.");
  }
  let injection;
  try {
    injection = await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      func: automationPageCommand,
      args: [command],
    });
  } catch {
    throw new Error(
      "이 탭을 조작할 수 없습니다. 주소창 옆 확장 아이콘을 눌러 연 팝업에서 다시 시도하세요.",
    );
  }
  const result = injection?.[0]?.result;
  if (!result) {
    throw new Error("화면 명령이 실패했습니다.");
  }
  if (result.ok !== true && !allowFailure) {
    throw new Error(
      result.reason === "cancelled" ? "선택을 취소했습니다." : "화면 명령이 실패했습니다.",
    );
  }
  return result;
}

function renderCandidates() {
  candidatesEl.replaceChildren();
  if (!picked) {
    return;
  }

  const options = picked.candidates.filter((candidate) => candidate.matchCount === 1);
  const fallback = picked.candidates.filter((candidate) => candidate.matchCount !== 1);
  const ordered = [...options, ...fallback];
  if (ordered.length === 0) {
    const message = document.createElement("p");
    message.className = "hint";
    message.textContent = "이 요소에서 쓸 수 있는 위치 후보를 찾지 못했습니다.";
    candidatesEl.append(message);
    return;
  }

  ordered.forEach((candidate, index) => {
    const label = document.createElement("label");
    label.className = "candidate";

    const radio = document.createElement("input");
    radio.type = "radio";
    radio.name = "candidate";
    radio.value = String(index);
    radio.checked = index === 0;

    const text = document.createElement("span");
    const unique = candidate.matchCount === 1 ? "고유" : `${candidate.matchCount}개 일치`;
    text.textContent = `${candidate.kind} · ${candidate.value} (${unique})`;

    label.append(radio, text);
    candidatesEl.append(label);
  });
}

function renderFields() {
  fieldsEl.replaceChildren();
  if (draftFields.length === 0) {
    const empty = document.createElement("li");
    empty.className = "hint";
    empty.textContent = "아직 추가한 필드가 없습니다.";
    fieldsEl.append(empty);
    return;
  }

  draftFields.forEach((field, index) => {
    const item = document.createElement("li");
    const text = document.createElement("span");
    text.textContent = `${field.label} · ${field.locator.kind} · ${field.locator.value}`;

    const remove = document.createElement("button");
    remove.type = "button";
    remove.className = "link";
    remove.textContent = "삭제";
    remove.addEventListener("click", () => {
      draftFields.splice(index, 1);
      renderFields();
    });

    item.append(text, remove);
    fieldsEl.append(item);
  });
}

function suggestionKindText(kind) {
  return SUGGESTION_KIND_TEXT[kind] ?? kind;
}

/** 규칙 기반 실행기가 검증한 레시피 후보를 이름 입력과 함께 보여 줍니다. */
function renderSuggestions(suggestions) {
  suggestionsEl.replaceChildren();
  if (suggestions.length === 0) {
    const empty = document.createElement("li");
    empty.className = "hint";
    empty.textContent = "이 화면에서 바로 실행할 수 있는 레시피를 찾지 못했습니다.";
    suggestionsEl.append(empty);
    return;
  }

  for (const suggestion of suggestions) {
    const item = document.createElement("li");
    item.className = "suggestion";

    const head = document.createElement("div");
    head.className = "suggestion-head";
    const badge = document.createElement("span");
    badge.className = "badge";
    badge.textContent = suggestionKindText(suggestion.kind);
    const summary = document.createElement("span");
    summary.className = "hint";
    summary.textContent = suggestion.summary;
    head.append(badge, summary);

    const nameInput = document.createElement("input");
    nameInput.type = "text";
    nameInput.maxLength = 80;
    nameInput.value = suggestion.name;
    nameInput.setAttribute("aria-label", "레시피 이름");

    const row = document.createElement("div");
    row.className = "row";
    const save = document.createElement("button");
    save.type = "button";
    save.textContent = "이 레시피 저장";
    save.addEventListener("click", () => saveSuggestion(suggestion, nameInput, save));
    row.append(save);

    item.append(head, nameInput, row);
    suggestionsEl.append(item);
  }
}

/**
 * 저장 직전에 화면을 다시 분석해 같은 레시피가 아직 성립하는지 확인합니다.
 * 화면이 바뀌었으면 저장하지 않습니다. 값은 여전히 보내지 않습니다.
 */
async function saveSuggestion(suggestion, nameInput, button) {
  const name = nameInput.value.trim();
  if (!name) {
    setStatus("레시피 이름을 입력하세요.", "error");
    return;
  }
  const path = SUGGESTION_PATHS[suggestion.kind];
  if (!path) {
    setStatus("저장할 수 없는 추천 종류입니다.", "error");
    return;
  }

  let saved = false;
  button.disabled = true;
  try {
    const tab = await activeTab();
    if (!tab?.url || !/^https?:/i.test(tab.url)) {
      setStatus("http/https 화면에서만 저장할 수 있습니다.", "error");
      return;
    }
    const fresh = await runPageCommand({ op: "suggest-recipes" });
    const current = fresh.suggestions.find((item) => item.key === suggestion.key);
    if (!current) {
      setStatus("화면이 바뀌어 저장하지 않았습니다. '화면 분석'을 다시 실행하세요.", "error");
      return;
    }
    const created = await bridgeFetch(path, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name, url: tab.url, ...current.payload }),
    });
    await refreshFillRecipes();
    const select =
      suggestion.kind === "fill"
        ? recipeSelect
        : suggestion.kind === "table_fill"
          ? tableSelect
          : watchSelect;
    select.value = created.id;
    renderValueInputs();
    saved = true;
    button.textContent = "저장됨";
    setStatus(
      `'${created.name}' ${suggestionKindText(suggestion.kind)} 레시피를 저장했습니다.`,
      "ok",
    );
  } catch (error) {
    setStatus(error.message, "error");
  } finally {
    if (!saved) {
      button.disabled = false;
    }
  }
}

function renderPreview(result) {
  previewResultsEl.replaceChildren();
  for (const field of result.results) {
    const item = document.createElement("li");
    if (field.matchCount === 1) {
      item.textContent = `${field.label}: 찾음 · 현재 값 "${field.currentValue ?? ""}"`;
      item.dataset.tone = "ok";
    } else if (field.matchCount === 0) {
      item.textContent = `${field.label}: 찾지 못함`;
      item.dataset.tone = "error";
    } else {
      item.textContent = `${field.label}: 모호함(${field.matchCount}개 일치)`;
      item.dataset.tone = "error";
    }
    previewResultsEl.append(item);
  }
}

/**
 * 선택한 레시피의 필드마다 값을 입력할 칸을 만듭니다. 값은 저장하지 않고
 * 실행할 때만 페이지로 보냅니다.
 */
function renderValueInputs() {
  valuesEl.replaceChildren();
  valueInputs = [];

  const recipe = fillRecipes.find((item) => item.id === recipeSelect.value);
  if (!recipe) {
    return;
  }

  for (const field of recipe.fields ?? []) {
    const row = document.createElement("label");
    row.className = "value-row";

    const caption = document.createElement("span");
    caption.textContent = field.label;

    const input = document.createElement("input");
    input.type = "text";
    input.autocomplete = "off";
    input.placeholder = "넣을 값";

    row.append(caption, input);
    valuesEl.append(row);
    valueInputs.push(input);
  }
}

function renderFillResults(result, failureReason) {
  fillResultsEl.replaceChildren();
  for (const field of result.results ?? []) {
    const item = document.createElement("li");
    if (failureReason === "locator_not_unique" || failureReason === "unsupported_control") {
      item.dataset.tone = "error";
      item.textContent =
        field.matchCount === 1
          ? `${field.label}: 입력할 수 없는 칸입니다`
          : `${field.label}: ${field.matchCount}개 일치`;
    } else {
      item.dataset.tone = field.verified ? "ok" : "error";
      item.textContent = field.verified
        ? `${field.label}: "${field.previousValue ?? ""}" -> "${field.readBack ?? ""}" 확인`
        : `${field.label}: 검증 실패("${field.readBack ?? ""}")`;
    }
    fillResultsEl.append(item);
  }
}

async function refreshFillRecipes() {
  let recipes;
  try {
    recipes = await bridgeFetch("/v1/bridge/recipes");
  } catch {
    return;
  }
  fillRecipes = (recipes ?? []).filter((recipe) => recipe.kind === "fill");
  tableRecipes = (recipes ?? []).filter((recipe) => recipe.kind === "table_fill");
  watchRecipes = (recipes ?? []).filter((recipe) => recipe.kind === "watch");

  const previous = recipeSelect.value;
  recipeSelect.replaceChildren();
  for (const recipe of fillRecipes) {
    const option = document.createElement("option");
    option.value = recipe.id;
    option.textContent = `${recipe.name} (필드 ${recipe.fields?.length ?? 0}개)`;
    recipeSelect.append(option);
  }
  if (fillRecipes.length === 0) {
    const option = document.createElement("option");
    option.value = "";
    option.textContent = "저장된 자동입력 레시피가 없습니다";
    recipeSelect.append(option);
  } else if (fillRecipes.some((recipe) => recipe.id === previous)) {
    recipeSelect.value = previous;
  }
  renderValueInputs();

  const previousTable = tableSelect.value;
  tableSelect.replaceChildren();
  for (const recipe of tableRecipes) {
    const option = document.createElement("option");
    option.value = recipe.id;
    option.textContent = `${recipe.name} (${recipe.table?.identityHeaders?.length ?? 0}열 · 날짜 ${
      recipe.table?.dateLabels?.length ?? 0
    }개)`;
    tableSelect.append(option);
  }
  if (tableRecipes.length === 0) {
    const option = document.createElement("option");
    option.value = "";
    option.textContent = "저장된 표 레시피가 없습니다";
    tableSelect.append(option);
  } else if (tableRecipes.some((recipe) => recipe.id === previousTable)) {
    tableSelect.value = previousTable;
  }

  const previousWatch = watchSelect.value;
  watchSelect.replaceChildren();
  for (const recipe of watchRecipes) {
    const option = document.createElement("option");
    option.value = recipe.id;
    option.textContent = `${recipe.name} (${recipe.watch?.list?.value ?? ""})`;
    watchSelect.append(option);
  }
  if (watchRecipes.length === 0) {
    const option = document.createElement("option");
    option.value = "";
    option.textContent = "저장된 감시 레시피가 없습니다";
    watchSelect.append(option);
  } else if (watchRecipes.some((recipe) => recipe.id === previousWatch)) {
    watchSelect.value = previousWatch;
  }
}

function selectedTableRecipe() {
  return tableRecipes.find((recipe) => recipe.id === tableSelect.value) ?? null;
}

function selectedWatchRecipe() {
  return watchRecipes.find((recipe) => recipe.id === watchSelect.value) ?? null;
}

/** 감시 명령이 중단한 이유를 사용자가 읽을 수 있는 문장으로 바꿉니다. */
function watchReasonText(reason) {
  const texts = {
    cancelled: "선택을 취소했습니다",
    row_missing: "클릭한 위치에서 목록 행을 찾지 못했습니다",
    identity_missing: "행 안에서 식별자 위치를 만들지 못했습니다",
    rows_missing: "목록 행을 찾지 못했습니다",
    login_required: "로그인 화면으로 바뀐 것 같습니다. 브라우저에서 다시 로그인하세요",
    identity_not_unique: "행 안에서 식별자를 하나로 특정하지 못했습니다",
    duplicate_identifiers: "식별자 값이 중복되는 행이 있습니다",
  };
  return texts[reason] ?? reason ?? "알 수 없는 이유";
}

function renderWatchResults(outcome) {
  watchResultsEl.replaceChildren();
  for (const identifier of outcome.newIdentifiers ?? []) {
    const item = document.createElement("li");
    item.dataset.tone = "ok";
    item.textContent = `새 항목: ${identifier}`;
    watchResultsEl.append(item);
  }
  const summary = document.createElement("li");
  summary.className = "hint";
  summary.textContent = `전체 ${outcome.total ?? 0}개 · 마지막 확인 ${new Date(
    outcome.checkedAtMs ?? Date.now(),
  ).toLocaleString()}`;
  watchResultsEl.append(summary);
}

/** 표 명령이 중단한 이유를 사용자가 읽을 수 있는 문장으로 바꿉니다. */
function tableReasonText(reason) {
  const texts = {
    table_not_unique: "표를 하나로 특정하지 못했습니다",
    table_missing: "클릭한 위치에서 표를 찾지 못했습니다",
    table_too_small: "표에 데이터 행이 없습니다",
    header_row_missing: "머리글 행을 찾지 못했습니다",
    identity_columns_missing: "행 식별 열(학년·반·이름 등)을 찾지 못했습니다",
    date_columns_missing: "날짜 열을 찾지 못했습니다",
    rows_missing: "데이터 행을 찾지 못했습니다",
    columns_changed: "저장한 머리글과 지금 표가 다릅니다",
    duplicate_rows: "행 식별 값이 중복된 행이 있습니다",
    unknown_rows: "미리보기 이후 행 식별 값이 바뀌었습니다",
    cell_not_writable: "입력할 수 없는 칸이 있습니다",
    nothing_to_fill: "입력할 값이 없습니다",
    locator_not_unique: "입력 칸을 하나로 특정하지 못했습니다",
    unsupported_control: "쓸 수 없는 컨트롤입니다",
  };
  return texts[reason] ?? reason ?? "알 수 없는 이유";
}

function renderTableResults(result) {
  tableResultsEl.replaceChildren();
  for (const field of result.results ?? []) {
    const item = document.createElement("li");
    item.dataset.tone = field.verified ? "ok" : "error";
    item.textContent = field.verified
      ? `${field.row} ${field.date}: "${field.readBack ?? ""}" 확인`
      : `${field.row} ${field.date}: 검증 실패("${field.readBack ?? ""}")`;
    tableResultsEl.append(item);
  }
  for (const issue of result.issues ?? []) {
    const item = document.createElement("li");
    item.dataset.tone = "error";
    item.textContent = `${issue.row} ${issue.date}: ${tableReasonText(issue.reason)}`;
    tableResultsEl.append(item);
  }
}

/**
 * 미리보기 결과를 탭으로 구분한 행렬로 보여 줍니다. 첫 줄은 머리글이고,
 * 앞쪽 열은 행 식별 값, 나머지는 날짜 열 값입니다.
 */
function renderTableMatrix(preview) {
  const lines = [[...preview.identityHeaders, ...preview.dateLabels].join("\t")];
  for (const row of preview.rows) {
    const values = row.values.map((value) => (value === null ? "" : value));
    lines.push([...row.identities, ...values].join("\t"));
  }
  tableValuesEl.value = lines.join("\n");
}

/**
 * 행렬 입력을 값 객체로 바꿉니다. 행 식별 값은 미리보기의 행과 정확히 같아야
 * 하므로, 바뀐 행은 실행 단계에서 거부됩니다.
 */
function tableValuesFromMatrix(recipe) {
  const identityCount = recipe.table?.identityHeaders?.length ?? 0;
  const dateLabels = recipe.table?.dateLabels ?? [];
  const lines = tableValuesEl.value.split(/\r?\n/).filter((line) => line.trim() !== "");
  const values = {};

  for (const line of lines.slice(1)) {
    const cells = line.split("\t");
    const identities = cells.slice(0, identityCount).map((value) => value.trim());
    if (identities.every((value) => !value)) {
      continue;
    }
    const key = identities.join("\u0000");
    const rowValues = {};
    dateLabels.forEach((label, index) => {
      const raw = cells[identityCount + index];
      if (typeof raw === "string" && raw.trim() !== "") {
        rowValues[label] = raw.trim();
      }
    });
    if (Object.keys(rowValues).length === 0) {
      continue;
    }
    values[key] = rowValues;
  }

  return values;
}

document.getElementById("save").addEventListener("click", async () => {
  const token = tokenInput.value.trim();
  if (!token) {
    setStatus("토큰을 입력하세요.", "error");
    return;
  }
  await chrome.storage.local.set({ token });
  setStatus("토큰을 저장했습니다. '연결 확인'으로 점검하세요.", "ok");
});

document.getElementById("check").addEventListener("click", async () => {
  try {
    const status = await bridgeFetch("/v1/bridge/status");
    setStatus(`연결됨 · ${status.service} ${status.version}`, "ok");
    await refreshFillRecipes();
  } catch (error) {
    setStatus(error.message, "error");
  }
});

document.getElementById("suggest").addEventListener("click", async () => {
  setStatus("현재 화면 구조를 확인하는 중입니다.", "");
  try {
    const result = await runPageCommand({ op: "suggest-recipes" });
    renderSuggestions(result.suggestions);
    setStatus(
      result.suggestions.length === 0
        ? "이 화면에서 바로 실행할 수 있는 레시피를 찾지 못했습니다."
        : `레시피 후보 ${result.suggestions.length}개를 찾았습니다. 이름을 확인하고 저장하세요.`,
      result.suggestions.length === 0 ? "" : "ok",
    );
  } catch (error) {
    suggestionsEl.replaceChildren();
    setStatus(error.message, "error");
  }
});

document.getElementById("register").addEventListener("click", async () => {
  try {
    const tab = await activeTab();
    if (!tab?.url || !/^https?:/i.test(tab.url)) {
      setStatus("http/https 화면에서만 등록할 수 있습니다.", "error");
      return;
    }
    const name = (nameInput.value.trim() || tab.title || "업무 화면").slice(0, 80);
    const created = await bridgeFetch("/v1/bridge/shortcuts", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name, url: tab.url }),
    });
    nameInput.value = "";
    setStatus(`'${created.name}' 버튼을 등록했습니다.`, "ok");
  } catch (error) {
    setStatus(error.message, "error");
  }
});

document.getElementById("pick").addEventListener("click", async () => {
  setStatus("화면에서 요소를 클릭하세요.", "");
  try {
    const result = await runPageCommand({ op: "pick" });
    picked = result;
    pickerEl.hidden = false;
    const summary = result.element.label || result.element.text || result.element.tag;
    pickedEl.textContent = `선택: ${result.element.tag} · ${summary}`;
    fieldLabelInput.value = (result.element.label || result.element.text || "").slice(0, 80);
    renderCandidates();
    setStatus("위치 후보를 고르고 필드 이름을 확인한 뒤 추가하세요.", "ok");
  } catch (error) {
    picked = null;
    pickerEl.hidden = true;
    setStatus(error.message, "error");
  }
});

document.getElementById("add-field").addEventListener("click", () => {
  if (!picked) {
    setStatus("먼저 화면에서 요소를 선택하세요.", "error");
    return;
  }
  const selected = candidatesEl.querySelector("input[name='candidate']:checked");
  const ordered = [
    ...picked.candidates.filter((candidate) => candidate.matchCount === 1),
    ...picked.candidates.filter((candidate) => candidate.matchCount !== 1),
  ];
  const candidate = selected ? ordered[Number(selected.value)] : null;
  if (!candidate) {
    setStatus("위치 후보를 고르세요.", "error");
    return;
  }

  const label = (fieldLabelInput.value.trim() || `필드 ${draftFields.length + 1}`).slice(0, 80);
  draftFields.push({ label, locator: { kind: candidate.kind, value: candidate.value } });
  picked = null;
  pickerEl.hidden = true;
  renderFields();
  setStatus(`'${label}' 필드를 추가했습니다.`, "ok");
});

document.getElementById("save-recipe").addEventListener("click", async () => {
  const recipeName = recipeNameInput.value.trim();
  if (!recipeName) {
    setStatus("레시피 이름을 입력하세요.", "error");
    return;
  }
  if (draftFields.length === 0) {
    setStatus("필드를 하나 이상 추가하세요.", "error");
    return;
  }

  try {
    const tab = await activeTab();
    if (!tab?.url || !/^https?:/i.test(tab.url)) {
      setStatus("http/https 화면에서만 저장할 수 있습니다.", "error");
      return;
    }
    const created = await bridgeFetch("/v1/bridge/fill-recipes", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name: recipeName, url: tab.url, fields: draftFields }),
    });
    draftFields = [];
    recipeNameInput.value = "";
    renderFields();
    await refreshFillRecipes();
    recipeSelect.value = created.id;
    setStatus(`'${created.name}' 자동입력 레시피를 저장했습니다(필드 ${created.fieldCount}개).`, "ok");
  } catch (error) {
    setStatus(error.message, "error");
  }
});

document.getElementById("preview").addEventListener("click", async () => {
  const recipe = fillRecipes.find((item) => item.id === recipeSelect.value);
  if (!recipe) {
    setStatus("미리 볼 자동입력 레시피를 고르세요.", "error");
    return;
  }

  try {
    const tab = await activeTab();
    if (!ensureRecipeScreen(recipe, tab, "미리 볼")) {
      return;
    }

    const result = await runPageCommand({ op: "preview", fields: recipe.fields ?? [] });
    renderPreview(result);
    const missing = result.results.filter((field) => field.matchCount !== 1).length;
    if (missing > 0) {
      setStatus(`미리보기: ${missing}개 필드가 모호하거나 없습니다. 입력을 실행하지 않습니다.`, "error");
    } else {
      setStatus(`미리보기: ${result.results.length}개 필드를 모두 찾았습니다.`, "ok");
    }
  } catch (error) {
    setStatus(error.message, "error");
  }
});

document.getElementById("fill").addEventListener("click", async () => {
  const recipe = fillRecipes.find((item) => item.id === recipeSelect.value);
  if (!recipe) {
    setStatus("실행할 자동입력 레시피를 고르세요.", "error");
    return;
  }

  try {
    const tab = await activeTab();
    if (!ensureRecipeScreen(recipe, tab, "실행할")) {
      return;
    }

    const fields = recipe.fields ?? [];
    const values = fields.map((_, index) => valueInputs[index]?.value ?? "");
    const result = await runPageCommand({ op: "fill", fields, values }, { allowFailure: true });
    renderFillResults(result, result.ok === true ? null : result.reason);

    if (result.ok !== true) {
      setStatus(
        result.reason === "unsupported_control"
          ? "입력할 수 없는 칸이 있어 아무것도 입력하지 않았습니다."
          : "없거나 여러 개 일치하는 필드가 있어 아무것도 입력하지 않았습니다.",
        "error",
      );
      return;
    }

    const failed = result.results.filter((field) => !field.verified).length;
    setStatus(
      failed === 0
        ? `${result.results.length}개 필드를 입력했습니다. 내용을 확인한 뒤 사이트에서 직접 저장하세요.`
        : `${failed}개 필드를 확인하지 못했습니다. 화면을 확인하세요.`,
      failed === 0 ? "ok" : "error",
    );
  } catch (error) {
    setStatus(error.message, "error");
  }
});

recipeSelect.addEventListener("change", renderValueInputs);

document.getElementById("pick-table").addEventListener("click", async () => {
  setStatus("표 안의 칸을 클릭하세요.", "");
  try {
    const result = await runPageCommand({ op: "pick-table" }, { allowFailure: true });
    if (result.ok !== true) {
      pickedTable = null;
      tablePickedEl.textContent = "";
      setStatus(`표를 등록하지 못했습니다: ${tableReasonText(result.reason)}.`, "error");
      return;
    }
    pickedTable = result.spec;
    tablePickedEl.textContent = `표: 행 식별 ${result.spec.identityHeaders.join(", ")} · 날짜 ${
      result.spec.dateLabels.length
    }개 · 데이터 행 ${result.rows.length}개${
      result.missingEditors > 0 ? ` · 입력 칸 없음 ${result.missingEditors}개` : ""
    }`;
    tableNameInput.value = tableNameInput.value || "";
    setStatus("레시피 이름을 넣고 저장하세요.", "ok");
  } catch (error) {
    pickedTable = null;
    tablePickedEl.textContent = "";
    setStatus(error.message, "error");
  }
});

document.getElementById("save-table-recipe").addEventListener("click", async () => {
  const name = tableNameInput.value.trim();
  if (!pickedTable) {
    setStatus("먼저 표를 등록하세요.", "error");
    return;
  }
  if (!name) {
    setStatus("레시피 이름을 입력하세요.", "error");
    return;
  }

  try {
    const tab = await activeTab();
    if (!tab?.url || !/^https?:/i.test(tab.url)) {
      setStatus("http/https 화면에서만 저장할 수 있습니다.", "error");
      return;
    }
    const created = await bridgeFetch("/v1/bridge/table-recipes", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name, url: tab.url, table: pickedTable }),
    });
    pickedTable = null;
    tablePickedEl.textContent = "";
    tableNameInput.value = "";
    await refreshFillRecipes();
    tableSelect.value = created.id;
    setStatus(
      `'${created.name}' 표 레시피를 저장했습니다(식별 ${created.identityHeaders}열 · 날짜 ${created.dateLabels}개).`,
      "ok",
    );
  } catch (error) {
    setStatus(error.message, "error");
  }
});

document.getElementById("table-preview").addEventListener("click", async () => {
  const recipe = selectedTableRecipe();
  if (!recipe) {
    setStatus("미리 볼 표 레시피를 고르세요.", "error");
    return;
  }

  try {
    const tab = await activeTab();
    if (!ensureRecipeScreen(recipe, tab, "미리 볼")) {
      return;
    }

    const preview = await runPageCommand(
      { op: "preview-table", table: recipe.table },
      { allowFailure: true },
    );
    if (preview.ok !== true) {
      tableResultsEl.replaceChildren();
      setStatus(`표 미리보기를 중단했습니다: ${tableReasonText(preview.reason)}.`, "error");
      return;
    }
    renderTableMatrix(preview);
    tableResultsEl.replaceChildren();
    setStatus(
      `표 미리보기: 데이터 행 ${preview.rows.length}개 · 날짜 ${
        preview.dateLabels.length
      }개${preview.missingEditors > 0 ? ` · 입력 칸 없음 ${preview.missingEditors}개` : ""}`,
      preview.missingEditors > 0 ? "error" : "ok",
    );
  } catch (error) {
    setStatus(error.message, "error");
  }
});

document.getElementById("table-fill").addEventListener("click", async () => {
  const recipe = selectedTableRecipe();
  if (!recipe) {
    setStatus("실행할 표 레시피를 고르세요.", "error");
    return;
  }

  try {
    const tab = await activeTab();
    if (!ensureRecipeScreen(recipe, tab, "실행할")) {
      return;
    }

    const values = tableValuesFromMatrix(recipe);
    if (Object.keys(values).length === 0) {
      setStatus("입력할 값이 없습니다. 먼저 표 미리보기로 행렬을 불러오세요.", "error");
      return;
    }

    const result = await runPageCommand(
      { op: "fill-table", table: recipe.table, values },
      { allowFailure: true },
    );
    renderTableResults(result);
    if (result.ok !== true) {
      setStatus(
        `표 입력을 중단했습니다: ${tableReasonText(result.reason)}. 아무것도 입력하지 않았습니다.`,
        "error",
      );
      return;
    }

    const failed = result.results.filter((field) => !field.verified).length;
    setStatus(
      failed === 0
        ? `${result.results.length}개 칸을 입력했습니다. 내용을 확인한 뒤 사이트에서 직접 저장하세요.`
        : `${failed}개 칸을 확인하지 못했습니다. 화면을 확인하세요.`,
      failed === 0 ? "ok" : "error",
    );
  } catch (error) {
    setStatus(error.message, "error");
  }
});

document.getElementById("pick-watch").addEventListener("click", async () => {
  setStatus("감시할 목록의 행 안에서 식별자(예: 신청번호)를 클릭하세요.", "");
  try {
    const result = await runPageCommand({ op: "pick-watch" }, { allowFailure: true });
    if (result.ok !== true) {
      pickedWatch = null;
      watchPickedEl.textContent = "";
      setStatus(`감시를 등록하지 못했습니다: ${watchReasonText(result.reason)}.`, "error");
      return;
    }
    pickedWatch = result.spec;
    watchPickedEl.textContent = `감시: 행 ${result.rowCount}개 · 식별 예: ${
      result.sample.join(", ") || "(없음)"
    }`;
    setStatus("레시피 이름을 넣고 저장하세요.", "ok");
  } catch (error) {
    pickedWatch = null;
    watchPickedEl.textContent = "";
    setStatus(error.message, "error");
  }
});

document.getElementById("save-watch-recipe").addEventListener("click", async () => {
  const name = watchNameInput.value.trim();
  if (!pickedWatch) {
    setStatus("먼저 감시할 목록을 등록하세요.", "error");
    return;
  }
  if (!name) {
    setStatus("레시피 이름을 입력하세요.", "error");
    return;
  }

  try {
    const tab = await activeTab();
    if (!tab?.url || !/^https?:/i.test(tab.url)) {
      setStatus("http/https 화면에서만 저장할 수 있습니다.", "error");
      return;
    }
    const created = await bridgeFetch("/v1/bridge/watch-recipes", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name, url: tab.url, watch: pickedWatch }),
    });
    pickedWatch = null;
    watchPickedEl.textContent = "";
    watchNameInput.value = "";
    await refreshFillRecipes();
    watchSelect.value = created.id;
    setStatus(`'${created.name}' 감시 레시피를 저장했습니다.`, "ok");
  } catch (error) {
    setStatus(error.message, "error");
  }
});

document.getElementById("watch-scan").addEventListener("click", async () => {
  const recipe = selectedWatchRecipe();
  if (!recipe) {
    setStatus("실행할 감시 레시피를 고르세요.", "error");
    return;
  }

  try {
    const tab = await activeTab();
    if (!ensureRecipeScreen(recipe, tab, "실행할")) {
      return;
    }

    const scan = await runPageCommand(
      { op: "watch-scan", watch: recipe.watch },
      { allowFailure: true },
    );
    if (scan.ok !== true) {
      watchResultsEl.replaceChildren();
      setStatus(`감시를 중단했습니다: ${watchReasonText(scan.reason)}.`, "error");
      return;
    }

    const outcome = await bridgeFetch("/v1/bridge/watch-scans", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ recipeId: recipe.id, identifiers: scan.identifiers }),
    });
    renderWatchResults(outcome);

    if (outcome.paused) {
      setStatus("감시가 중지되어 있습니다. 앱에서 다시 켜세요.", "error");
      return;
    }
    setStatus(
      outcome.newIdentifiers.length === 0
        ? `새 항목이 없습니다(전체 ${outcome.total}개).`
        : `새 항목 ${outcome.newIdentifiers.length}개를 찾았습니다(전체 ${outcome.total}개).`,
      "ok",
    );
  } catch (error) {
    setStatus(error.message, "error");
  }
});

loadToken().then(refreshFillRecipes);
renderFields();
