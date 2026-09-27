const BRIDGE_URL = "http://127.0.0.1:43110";

const tokenInput = document.getElementById("token");
const nameInput = document.getElementById("name");
const statusEl = document.getElementById("status");

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
  } catch (error) {
    setStatus(error.message, "error");
  }
});

document.getElementById("register").addEventListener("click", async () => {
  try {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
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

loadToken();
