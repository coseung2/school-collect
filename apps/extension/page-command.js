/**
 * 페이지에서 실행하는 고정 명령입니다.
 *
 * chrome.scripting.executeScript가 이 함수 하나만 주입하고, 인자로는 데이터(command)만
 * 전달합니다. 임의 JavaScript 문자열이나 자유 형식 selector를 실행하지 않습니다.
 * 함수 안에서 바깥 스코프를 참조하지 않아야 페이지로 그대로 전달됩니다.
 */
export function automationPageCommand(command) {
  if (!command || typeof command !== "object") {
    return { ok: false, reason: "invalid_command" };
  }

  const CONTROL_TAGS = ["input", "select", "textarea"];

  function tagOf(element) {
    return element && element.tagName ? element.tagName.toLowerCase() : "";
  }

  function isFormControl(element) {
    return CONTROL_TAGS.indexOf(tagOf(element)) !== -1;
  }

  function readValue(element) {
    if (isFormControl(element)) {
      return typeof element.value === "string" ? element.value : "";
    }
    return (element.textContent || "").trim();
  }

  function labelTextFor(element) {
    if (element.labels && element.labels.length === 1) {
      const text = (element.labels[0].textContent || "").trim();
      if (text) {
        return text;
      }
    }
    const aria = (element.getAttribute("aria-label") || "").trim();
    if (aria) {
      return aria;
    }
    const labelledBy = element.getAttribute("aria-labelledby");
    if (labelledBy) {
      const target = document.getElementById(labelledBy);
      const text = target ? (target.textContent || "").trim() : "";
      if (text) {
        return text;
      }
    }
    return "";
  }

  function resolveAll(kind, value) {
    if (kind === "id") {
      const element = document.getElementById(value);
      return element ? [element] : [];
    }
    if (kind === "name") {
      return Array.from(document.getElementsByName(value));
    }
    if (kind === "label") {
      return Array.from(document.querySelectorAll("input, select, textarea")).filter(
        (control) => labelTextFor(control) === value,
      );
    }
    if (kind === "css") {
      try {
        return Array.from(document.querySelectorAll(value));
      } catch {
        return [];
      }
    }
    return [];
  }

  function safeIdValue(id) {
    return typeof id === "string" && /^[A-Za-z0-9_.:-]+$/.test(id) ? id : "";
  }

  function cssPathFor(element) {
    const parts = [];
    let node = element;
    let depth = 0;
    while (node && node.nodeType === 1 && depth < 6) {
      const tag = tagOf(node);
      let part = tag;
      const parent = node.parentElement;
      if (parent) {
        const siblings = Array.from(parent.children).filter(
          (child) => child.tagName === node.tagName,
        );
        if (siblings.length > 1) {
          part += ":nth-of-type(" + (siblings.indexOf(node) + 1) + ")";
        }
      }
      parts.unshift(part);

      const id = safeIdValue(node.id);
      if (id) {
        parts.unshift("#" + id);
        break;
      }
      node = parent;
      depth += 1;
    }
    return parts.join(" > ");
  }

  function candidateLocators(element) {
    const candidates = [];
    const seen = {};

    function push(kind, value) {
      const trimmed = typeof value === "string" ? value.trim() : "";
      if (!trimmed) {
        return;
      }
      const key = kind + "\u0000" + trimmed;
      if (seen[key]) {
        return;
      }
      seen[key] = true;
      candidates.push({ kind, value: trimmed, matchCount: resolveAll(kind, trimmed).length });
    }

    push("id", safeIdValue(element.id));
    if (isFormControl(element)) {
      push("name", element.getAttribute("name") || "");
      push("label", labelTextFor(element));
    }
    push("css", cssPathFor(element));
    return candidates;
  }

  function describeElement(element) {
    return {
      tag: tagOf(element),
      isFormControl: isFormControl(element),
      label: labelTextFor(element).slice(0, 80),
      text: (element.textContent || "").trim().slice(0, 60),
    };
  }

  function pickElement() {
    return new Promise((resolve) => {
      let current = null;

      const overlay = document.createElement("div");
      overlay.style.cssText =
        "position:fixed;z-index:2147483647;pointer-events:none;border:2px solid #1f5fbf;background:rgba(31,95,191,0.12);border-radius:4px;";
      const hint = document.createElement("div");
      hint.textContent = "등록할 입력 칸을 클릭하세요. (Esc 취소)";
      hint.style.cssText =
        "position:fixed;left:50%;top:12px;transform:translateX(-50%);z-index:2147483647;pointer-events:none;background:#111827;color:#ffffff;font:12px/1.4 system-ui;padding:6px 10px;border-radius:6px;";
      document.documentElement.appendChild(overlay);
      document.documentElement.appendChild(hint);

      function highlight(element) {
        const rect = element.getBoundingClientRect();
        overlay.style.left = rect.left + "px";
        overlay.style.top = rect.top + "px";
        overlay.style.width = rect.width + "px";
        overlay.style.height = rect.height + "px";
      }

      function cleanup() {
        window.removeEventListener("mousemove", onMove, true);
        window.removeEventListener("click", onClick, true);
        window.removeEventListener("keydown", onKey, true);
        overlay.remove();
        hint.remove();
      }

      function onMove(event) {
        const element = event.target;
        if (!element || element === overlay || element === hint) {
          return;
        }
        current = element;
        highlight(element);
      }

      function onClick(event) {
        if (!current) {
          return;
        }
        event.preventDefault();
        event.stopPropagation();
        const element = current;
        cleanup();
        resolve({
          ok: true,
          element: describeElement(element),
          candidates: candidateLocators(element),
        });
      }

      function onKey(event) {
        if (event.key !== "Escape") {
          return;
        }
        event.preventDefault();
        cleanup();
        resolve({ ok: false, reason: "cancelled" });
      }

      window.addEventListener("mousemove", onMove, true);
      window.addEventListener("click", onClick, true);
      window.addEventListener("keydown", onKey, true);
    });
  }

  if (command.op === "pick") {
    return pickElement();
  }

  if (command.op === "preview") {
    const fields = Array.isArray(command.fields) ? command.fields : [];
    const results = fields.map((field, index) => {
      const locator = field && field.locator ? field.locator : {};
      const matches = resolveAll(locator.kind, locator.value);
      const result = {
        index,
        label: (field && field.label) || "",
        matchCount: matches.length,
      };
      if (matches.length === 1) {
        result.currentValue = readValue(matches[0]);
      }
      return result;
    });
    return { ok: true, url: location.href, title: document.title, results };
  }

  return { ok: false, reason: "unsupported_op" };
}
