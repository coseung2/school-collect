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
    if (tagOf(element) === "input") {
      const type = (element.type || "text").toLowerCase();
      if (type === "checkbox" || type === "radio") {
        return String(element.checked);
      }
    }
    if (isFormControl(element)) {
      return typeof element.value === "string" ? element.value : "";
    }
    return (element.textContent || "").trim();
  }

  const WRITABLE_INPUT_TYPES = [
    "text",
    "search",
    "url",
    "tel",
    "email",
    "number",
    "date",
    "datetime-local",
    "month",
    "week",
    "time",
    "checkbox",
    "radio",
  ];

  function isWritable(element) {
    const tag = tagOf(element);
    if (tag === "textarea" || tag === "select") {
      return true;
    }
    if (tag !== "input") {
      return false;
    }
    return WRITABLE_INPUT_TYPES.indexOf((element.type || "text").toLowerCase()) !== -1;
  }

  function writeValue(element, value) {
    const tag = tagOf(element);
    const text = typeof value === "string" ? value : "";

    if (tag === "select") {
      const options = Array.from(element.options || []);
      const match =
        options.filter((option) => option.value === text)[0] ||
        options.filter((option) => (option.textContent || "").trim() === text)[0];
      if (!match) {
        return false;
      }
      element.value = match.value;
    } else if (tag === "input" && (element.type === "checkbox" || element.type === "radio")) {
      const desired = ["true", "1", "y", "yes", "on"].indexOf(text.toLowerCase()) !== -1;
      if (element.checked !== desired) {
        element.click();
      }
      if (element.checked !== desired) {
        return false;
      }
    } else if (tag === "textarea") {
      const descriptor = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value");
      if (descriptor && descriptor.set) {
        descriptor.set.call(element, text);
      } else {
        element.value = text;
      }
    } else if (tag === "input") {
      const descriptor = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value");
      if (descriptor && descriptor.set) {
        descriptor.set.call(element, text);
      } else {
        element.value = text;
      }
    } else {
      return false;
    }

    element.dispatchEvent(new Event("input", { bubbles: true }));
    element.dispatchEvent(new Event("change", { bubbles: true }));
    return true;
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

  /** 클릭한 요소가 속한 목록 행입니다. 감시는 행 단위로 식별자를 읽습니다. */
  function rowOf(element) {
    if (!element || !element.closest) {
      return null;
    }
    return element.closest("tr, li, [role='row']");
  }

  /**
   * 행 안에서의 상대 위치를 만듭니다. 목록 위치(css)와 달리 행 안쪽만
   * 가리키므로 저장 후에도 같은 열을 다시 찾을 수 있습니다.
   */
  function relativeSelector(element, row) {
    if (!element || element === row) {
      return null;
    }
    const id = safeIdValue(element.id);
    if (id) {
      return { kind: "id", value: id };
    }
    if (isFormControl(element) && typeof element.name === "string" && element.name.trim()) {
      return { kind: "name", value: element.name.trim() };
    }

    const parts = [];
    let node = element;
    let depth = 0;
    while (node && node !== row && depth < 6) {
      const parent = node.parentElement;
      if (!parent) {
        return null;
      }
      const tag = tagOf(node);
      const siblings = Array.from(parent.children).filter(
        (child) => child.tagName === node.tagName,
      );
      parts.unshift(
        siblings.length > 1 ? `${tag}:nth-of-type(${siblings.indexOf(node) + 1})` : tag,
      );
      node = parent;
      depth += 1;
    }
    return parts.length > 0 ? { kind: "css", value: parts.join(" > ") } : null;
  }

  /** 저장된 상대 위치를 주어진 행 안에서 다시 찾습니다. */
  function resolveWithin(root, kind, value) {
    if (!root || typeof value !== "string" || !value.trim()) {
      return [];
    }
    const text = value.trim();
    if (kind === "id") {
      const id = safeIdValue(text);
      return id ? Array.from(root.querySelectorAll(`[id="${id}"]`)) : [];
    }
    if (kind === "name") {
      return Array.from(root.querySelectorAll(`[name="${text}"]`));
    }
    if (kind === "label") {
      return Array.from(root.querySelectorAll("input, select, textarea")).filter(
        (control) => labelTextFor(control) === text,
      );
    }
    if (kind === "css") {
      try {
        return Array.from(root.querySelectorAll(text));
      } catch {
        return [];
      }
    }
    return [];
  }

  function identifierTextOf(element) {
    return readValue(element).replace(/\s+/g, " ").trim();
  }

  /** 로그인 화면으로 바뀌었는지 확인합니다. 감시 실패와 구분해 알리기 위한 것입니다. */
  function loginFormPresent() {
    return Boolean(document.querySelector('input[type="password"]'));
  }

  /**
   * 감시 목록을 읽어 식별자만 모읍니다. 본문(이름·사유 등)은 읽지 않습니다.
   * 행이 없거나 식별자를 하나로 특정하지 못하면 중단합니다.
   */
  function evaluateWatch(spec) {
    const list = (spec && spec.list) || {};
    const identity = (spec && spec.identity) || {};
    const rows = resolveAll(list.kind, list.value);
    if (rows.length === 0) {
      return {
        ok: false,
        reason: loginFormPresent() ? "login_required" : "rows_missing",
        rowCount: 0,
      };
    }

    const identifiers = [];
    let unreadable = 0;
    for (const row of rows) {
      const matches = resolveWithin(row, identity.kind, identity.value);
      if (matches.length !== 1) {
        unreadable += 1;
        continue;
      }
      const text = identifierTextOf(matches[0]);
      if (text) {
        identifiers.push(text);
      }
    }
    if (unreadable > 0) {
      return { ok: false, reason: "identity_not_unique", rowCount: rows.length, unreadable };
    }

    const seen = {};
    const duplicates = [];
    for (const identifier of identifiers) {
      if (seen[identifier]) {
        duplicates.push(identifier);
      }
      seen[identifier] = true;
    }
    if (duplicates.length > 0) {
      return {
        ok: false,
        reason: "duplicate_identifiers",
        rowCount: rows.length,
        duplicates: duplicates.slice(0, 5),
      };
    }

    return { ok: true, identifiers, rowCount: rows.length };
  }

  function cssPathFor(element) {
    const parts = [];
    let node = element;
    let depth = 0;
    while (node && node.nodeType === 1 && depth < 6) {
      const id = safeIdValue(node.id);
      if (id) {
        parts.unshift("#" + id);
        break;
      }

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

  const IDENTITY_HEADER_WORDS = [
    "학년도",
    "학년",
    "학급",
    "반",
    "번호",
    "번",
    "순번",
    "학번",
    "이름",
    "성명",
    "no",
    "name",
  ];
  const DATE_HEADER_PATTERN = /^[0-9]{1,2}([.\-/][0-9]{1,2})?\.?$/;

  function headerTextOf(cell) {
    return ((cell && cell.textContent) || "").replace(/\s+/g, " ").trim();
  }

  function editorsIn(cell) {
    if (!cell) {
      return [];
    }
    return Array.from(cell.querySelectorAll("input, select, textarea")).filter((element) => {
      const type = (element.type || "").toLowerCase();
      return !element.disabled && type !== "hidden";
    });
  }

  function analyzeTableElement(table) {
    const rows = Array.from(table.querySelectorAll("tr"));
    if (rows.length < 2) {
      return { ok: false, reason: "table_too_small" };
    }

    let headerRowIndex = -1;
    for (let index = 0; index < rows.length; index += 1) {
      if (rows[index].querySelectorAll("th").length >= 2) {
        headerRowIndex = index;
        break;
      }
    }
    if (headerRowIndex === -1) {
      return { ok: false, reason: "header_row_missing" };
    }

    const headers = Array.from(rows[headerRowIndex].children).map(headerTextOf);

    const identityColumns = [];
    for (let index = 0; index < headers.length; index += 1) {
      const text = headers[index].toLowerCase();
      if (!text) {
        continue;
      }
      const matched = IDENTITY_HEADER_WORDS.some(
        (word) => text === word || text.indexOf(word) === 0,
      );
      if (!matched) {
        break;
      }
      identityColumns.push(index);
    }
    if (identityColumns.length === 0) {
      return { ok: false, reason: "identity_columns_missing" };
    }

    const dateColumns = [];
    for (let index = 0; index < headers.length; index += 1) {
      if (identityColumns.indexOf(index) !== -1) {
        continue;
      }
      if (DATE_HEADER_PATTERN.test(headers[index])) {
        dateColumns.push(index);
      }
    }
    if (dateColumns.length === 0) {
      return { ok: false, reason: "date_columns_missing" };
    }

    const dataRows = [];
    let missingEditors = 0;
    for (let index = headerRowIndex + 1; index < rows.length; index += 1) {
      const cells = Array.from(rows[index].children);
      const identities = identityColumns.map((column) => headerTextOf(cells[column]));
      if (identities.every((value) => !value)) {
        continue;
      }
      const editors = dateColumns.map((column) => editorsIn(cells[column]));
      missingEditors += editors.filter((list) => list.length !== 1).length;
      dataRows.push({ rowIndex: index, cells, identities, editors });
    }
    if (dataRows.length === 0) {
      return { ok: false, reason: "rows_missing" };
    }

    return {
      ok: true,
      headers,
      identityColumns,
      dateColumns,
      dataRows,
      missingEditors,
      identityHeaders: identityColumns.map((column) => headers[column]),
      dateLabels: dateColumns.map((column) => headers[column]),
    };
  }

  /**
   * 저장된 머리글로 지금 표의 열을 다시 찾습니다. 같은 머리글이 두 번 나오거나
   * 없는 머리글이 있으면 중단합니다.
   */
  function matchStoredColumns(analysis, stored) {
    function uniqueIndex(label) {
      const matches = analysis.headers
        .map((header, index) => (header === label ? index : -1))
        .filter((index) => index !== -1);
      return matches.length === 1 ? matches[0] : -1;
    }

    const identityColumns = (stored.identityHeaders || []).map(uniqueIndex);
    const dateColumns = (stored.dateLabels || []).map(uniqueIndex);
    if (
      identityColumns.length === 0 ||
      dateColumns.length === 0 ||
      identityColumns.some((index) => index === -1) ||
      dateColumns.some((index) => index === -1)
    ) {
      return null;
    }
    return { identityColumns, dateColumns };
  }

  function rowsForColumns(analysis, columns) {
    const rows = [];
    let missingEditors = 0;
    for (const row of analysis.dataRows) {
      const identities = columns.identityColumns.map((column) => headerTextOf(row.cells[column]));
      if (identities.every((value) => !value)) {
        continue;
      }
      const editors = columns.dateColumns.map((column) => editorsIn(row.cells[column]));
      missingEditors += editors.filter((list) => list.length !== 1).length;
      rows.push({ identities, editors });
    }
    return { rows, missingEditors };
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
        resolve(element);
      }

      function onKey(event) {
        if (event.key !== "Escape") {
          return;
        }
        event.preventDefault();
        cleanup();
        resolve(null);
      }

      window.addEventListener("mousemove", onMove, true);
      window.addEventListener("click", onClick, true);
      window.addEventListener("keydown", onKey, true);
    });
  }

  if (command.op === "pick") {
    return pickElement().then((element) =>
      element
        ? {
            ok: true,
            element: describeElement(element),
            candidates: candidateLocators(element),
          }
        : { ok: false, reason: "cancelled" },
    );
  }

  if (command.op === "pick-table") {
    return pickElement().then((element) => {
      if (!element) {
        return { ok: false, reason: "cancelled" };
      }
      const table = element.closest ? element.closest("table") : null;
      if (!table) {
        return { ok: false, reason: "table_missing" };
      }
      const analysis = analyzeTableElement(table);
      if (analysis.ok !== true) {
        return analysis;
      }
      return {
        ok: true,
        spec: {
          locator: { kind: "css", value: cssPathFor(table) },
          identityHeaders: analysis.identityHeaders,
          dateLabels: analysis.dateLabels,
        },
        headers: analysis.headers,
        rows: analysis.dataRows.map((row) => row.identities),
        totalCells: analysis.dataRows.length * analysis.dateColumns.length,
        missingEditors: analysis.missingEditors,
      };
    });
  }

  if (command.op === "pick-watch") {
    return pickElement().then((element) => {
      if (!element) {
        return { ok: false, reason: "cancelled" };
      }
      const row = rowOf(element);
      if (!row || !row.parentElement) {
        return { ok: false, reason: "row_missing" };
      }
      const identity = relativeSelector(element, row);
      if (!identity) {
        return { ok: false, reason: "identity_missing" };
      }
      const list = {
        kind: "css",
        value: `${cssPathFor(row.parentElement)} > ${tagOf(row)}`,
      };

      const evaluated = evaluateWatch({ list, identity });
      if (evaluated.ok !== true) {
        return evaluated;
      }
      return {
        ok: true,
        spec: { list, identity },
        rowCount: evaluated.rowCount,
        sample: evaluated.identifiers.slice(0, 3),
      };
    });
  }

  if (command.op === "watch-scan") {
    const evaluated = evaluateWatch(command.watch);
    if (evaluated.ok !== true) {
      return evaluated;
    }
    return {
      ok: true,
      identifiers: evaluated.identifiers,
      rowCount: evaluated.rowCount,
      url: location.href,
      title: document.title,
    };
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

  if (command.op === "fill") {
    const fields = Array.isArray(command.fields) ? command.fields : [];
    const values = Array.isArray(command.values) ? command.values : [];
    const resolved = [];
    const results = fields.map((field, index) => {
      const locator = field && field.locator ? field.locator : {};
      const matches = resolveAll(locator.kind, locator.value);
      const result = {
        index,
        label: (field && field.label) || "",
        matchCount: matches.length,
      };
      if (matches.length === 1) {
        result.previousValue = readValue(matches[0]);
        if (!isWritable(matches[0])) {
          result.unsupported = true;
        }
        resolved.push({ element: matches[0], index });
      }
      return result;
    });

    // 하나라도 모호하거나 쓸 수 없으면 아무것도 입력하지 않습니다.
    if (results.some((result) => result.matchCount !== 1 || result.unsupported)) {
      return {
        ok: false,
        reason: results.some((result) => result.unsupported)
          ? "unsupported_control"
          : "locator_not_unique",
        results,
      };
    }

    for (const item of resolved) {
      const desired = typeof values[item.index] === "string" ? values[item.index] : "";
      const written = writeValue(item.element, desired);
      const result = results[item.index];
      result.writtenValue = desired;
      result.readBack = readValue(item.element);
      result.verified = written && result.readBack === desired;
    }

    return { ok: true, url: location.href, title: document.title, results };
  }

  if (command.op === "preview-table" || command.op === "fill-table") {
    const stored = command.table || {};
    const locator = stored.locator || {};
    const tables = resolveAll(locator.kind, locator.value);
    if (tables.length !== 1) {
      return { ok: false, reason: "table_not_unique", tableCount: tables.length };
    }

    const analysis = analyzeTableElement(tables[0]);
    if (analysis.ok !== true) {
      return analysis;
    }
    const columns = matchStoredColumns(analysis, stored);
    if (!columns) {
      return { ok: false, reason: "columns_changed" };
    }

    const { rows, missingEditors } = rowsForColumns(analysis, columns);
    if (rows.length === 0) {
      return { ok: false, reason: "rows_missing" };
    }

    const keys = rows.map((row) => row.identities.join("\u0000"));
    const duplicateKeys = keys.filter((key, index) => keys.indexOf(key) !== index);
    if (duplicateKeys.length > 0) {
      return { ok: false, reason: "duplicate_rows", duplicates: duplicateKeys.slice(0, 5) };
    }

    const dateLabels = stored.dateLabels || [];
    const preview = {
      ok: true,
      url: location.href,
      title: document.title,
      identityHeaders: stored.identityHeaders || [],
      dateLabels,
      missingEditors,
      rows: rows.map((row, index) => ({
        key: keys[index],
        identities: row.identities,
        values: row.editors.map((list) => (list.length === 1 ? readValue(list[0]) : null)),
      })),
    };

    if (command.op === "preview-table") {
      return preview;
    }

    const values =
      command.values && typeof command.values === "object" ? command.values : {};
    const unknownRows = Object.keys(values).filter((key) => keys.indexOf(key) === -1);
    if (unknownRows.length > 0) {
      return { ok: false, reason: "unknown_rows", unknownRows: unknownRows.slice(0, 5) };
    }

    // 먼저 입력 계획을 모두 세웁니다. 하나라도 쓸 수 없으면 아무것도 입력하지 않습니다.
    const plan = [];
    const issues = [];
    for (let rowIndex = 0; rowIndex < rows.length; rowIndex += 1) {
      const rowValues = values[keys[rowIndex]];
      if (!rowValues) {
        continue;
      }
      for (let columnIndex = 0; columnIndex < rows[rowIndex].editors.length; columnIndex += 1) {
        const label = dateLabels[columnIndex];
        const desired = rowValues[label];
        if (typeof desired !== "string") {
          continue;
        }
        const editors = rows[rowIndex].editors[columnIndex];
        const rowLabel = rows[rowIndex].identities.join(" ");
        if (editors.length !== 1) {
          issues.push({ row: rowLabel, date: label, reason: "locator_not_unique" });
          continue;
        }
        if (!isWritable(editors[0])) {
          issues.push({ row: rowLabel, date: label, reason: "unsupported_control" });
          continue;
        }
        plan.push({
          rowIndex,
          columnIndex,
          rowLabel,
          date: label,
          element: editors[0],
          desired,
          previousValue: readValue(editors[0]),
        });
      }
    }
    if (issues.length > 0) {
      return { ok: false, reason: "cell_not_writable", issues, planned: plan.length };
    }
    if (plan.length === 0) {
      return { ok: false, reason: "nothing_to_fill" };
    }

    const results = plan.map((item) => {
      const written = writeValue(item.element, item.desired);
      const readBack = readValue(item.element);
      return {
        row: item.rowLabel,
        date: item.date,
        previousValue: item.previousValue,
        writtenValue: item.desired,
        readBack,
        verified: written && readBack === item.desired,
      };
    });
    return { ok: true, url: location.href, title: document.title, results };
  }

  return { ok: false, reason: "unsupported_op" };
}
