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

let picked = null;
let draftFields = [];
let fillRecipes = [];

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

async function runPageCommand(command) {
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
  if (!result || result.ok !== true) {
    throw new Error(result?.reason === "cancelled" ? "선택을 취소했습니다." : "화면 명령이 실패했습니다.");
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

async function refreshFillRecipes() {
  let recipes;
  try {
    recipes = await bridgeFetch("/v1/bridge/recipes");
  } catch {
    return;
  }
  fillRecipes = (recipes ?? []).filter((recipe) => recipe.kind === "fill");

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
    if (!tab?.url || !/^https?:/i.test(tab.url)) {
      setStatus("http/https 화면에서만 미리 볼 수 있습니다.", "error");
      return;
    }
    const recipeUrl = new URL(recipe.targetUrl);
    const tabUrl = new URL(tab.url);
    if (recipeUrl.origin !== tabUrl.origin) {
      setStatus(`이 레시피는 ${recipeUrl.origin} 화면용입니다.`, "error");
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

loadToken().then(refreshFillRecipes);
renderFields();
