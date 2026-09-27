use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Mutex,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::Manager;
use tokio::sync::oneshot;
use url::Url;

mod bridge;
mod browser_login;
mod drafts;
mod pkce;

pub(crate) const AUTOMATION_RECIPES_FILE: &str = "automation-recipes.tsv";
const AUTOMATION_RECIPES_QUARANTINE_FILE: &str = "automation-recipes.invalid.tsv";
const MAX_RECIPE_NAME_LEN: usize = 80;
const MAX_RECIPE_ID_LEN: usize = 128;
const MAX_TARGET_URL_LEN: usize = 2048;
/// 자동입력 레시피 하나가 채울 수 있는 필드 수입니다.
const MAX_RECIPE_FIELDS: usize = 40;
const MAX_FIELD_LABEL_LEN: usize = 80;
const MAX_LOCATOR_VALUE_LEN: usize = 300;
/// 표 입력 레시피가 가질 수 있는 행 식별 머리글과 날짜 열 수입니다.
const MAX_TABLE_HEADERS: usize = 40;
const MAX_TABLE_DATES: usize = 62;
/// 감시 레시피가 보관할 수 있는 식별자 수와 한 식별자의 길이입니다.
const MAX_WATCH_IDENTIFIERS: usize = 500;
const MAX_WATCH_IDENTIFIER_LEN: usize = 120;
/// 감시 상태(마지막으로 본 식별자와 확인 시각)를 보관하는 파일입니다.
pub(crate) const AUTOMATION_WATCH_STATE_FILE: &str = "automation-watch-state.json";
const AUTOMATION_WATCH_STATE_QUARANTINE_FILE: &str = "automation-watch-state.invalid.json";
/// 감시 상태 파일 상한입니다. 넘으면 새 스캔 결과를 저장하지 않습니다.
const MAX_WATCH_STATE_BYTES: usize = 512 * 1024;
/// 한 사용자 계정이 보관할 수 있는 업무 버튼 수입니다.
const MAX_RECIPES: usize = 100;
/// 손상된 행을 보관하는 격리 파일의 상한입니다. 넘으면 복구를 멈추고 원본을 보존합니다.
const MAX_QUARANTINE_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AutomationKind {
    Shortcut,
    Fill,
    TableFill,
    Watch,
}

/// 화면 요소를 찾는 방법입니다.
///
/// 임의 JavaScript나 자유 형식 XPath를 받지 않도록 종류를 고정하고,
/// 값은 각 종류의 문법에 맞는지 검사합니다.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LocatorKind {
    /// 요소 id
    Id,
    /// 폼 컨트롤 name 속성
    Name,
    /// label 또는 aria-label 텍스트
    Label,
    /// 제한된 문자만 허용하는 CSS selector
    Css,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AutomationLocator {
    pub(crate) kind: LocatorKind,
    pub(crate) value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AutomationField {
    pub(crate) label: String,
    pub(crate) locator: AutomationLocator,
}

/// 학생 × 날짜 행렬처럼 표를 채우는 레시피의 표 정보입니다.
///
/// 열 위치(인덱스)는 저장하지 않고 머리글 텍스트만 저장합니다. 화면이 바뀌어
/// 열 순서가 달라져도 머리글로 다시 찾고, 못 찾으면 중단합니다.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AutomationTable {
    /// 표 또는 표를 담은 컨테이너의 위치
    pub(crate) locator: AutomationLocator,
    /// 행을 식별하는 머리글(예: 학년, 반, 번호, 이름)
    pub(crate) identity_headers: Vec<String>,
    /// 값을 넣을 날짜 열 머리글(예: 3-2, 03.02)
    pub(crate) date_labels: Vec<String>,
}

/// 새 항목을 찾을 목록의 행 위치와, 행 안에서 식별자를 읽을 위치입니다.
///
/// 감시는 목록 전체를 저장하지 않고 식별자 집합만 비교합니다.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AutomationWatch {
    /// 목록의 각 행 위치(예: `#list > tbody > tr`). css만 허용합니다.
    pub(crate) list: AutomationLocator,
    /// 행 안에서 식별자를 읽을 요소 위치(예: `td:nth-child(1)`).
    pub(crate) identity: AutomationLocator,
}

/// 저장 파일의 5번째 열에 들어가는 JSON입니다.
///
/// 이전 형식(필드 배열만 저장)도 계속 읽을 수 있도록 `decode_recipe`가 둘 다 받습니다.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AutomationPayload {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) fields: Vec<AutomationField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) table: Option<AutomationTable>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) watch: Option<AutomationWatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AutomationRecipe {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) target_url: String,
    pub(crate) kind: AutomationKind,
    /// 자동입력 레시피가 채울 필드입니다. 바로가기는 비어 있습니다.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) fields: Vec<AutomationField>,
    /// 표 입력 레시피의 표 정보입니다. 그 밖의 종류는 값을 갖지 않습니다.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) table: Option<AutomationTable>,
    /// 감시 레시피의 목록·식별자 위치입니다. 그 밖의 종류는 값을 갖지 않습니다.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) watch: Option<AutomationWatch>,
}

fn validate_plain_field(value: &str) -> Result<(), String> {
    if value.contains(['\t', '\r', '\n']) {
        return Err("탭이나 줄바꿈 문자는 사용할 수 없습니다.".to_string());
    }
    Ok(())
}

/// 저장·실행에 쓸 주소를 해석합니다.
///
/// `url` crate를 쓰면 스킴이 소문자로 정규화되고 기본 포트가 정리되므로,
/// `HTTPS://` 같은 표기와 `https://:8080` 같은 호스트 없는 값이 일관되게 처리됩니다.
fn parse_target_url(value: &str) -> Result<Url, String> {
    if value.len() > MAX_TARGET_URL_LEN {
        return Err("대상 URL이 너무 깁니다.".to_string());
    }
    validate_plain_field(value)?;

    let url = Url::parse(value).map_err(|_| "대상 URL을 해석하지 못했습니다.".to_string())?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("http 또는 https URL만 등록할 수 있습니다.".to_string());
    }
    if url.host_str().is_none() {
        return Err("대상 URL에 호스트가 없습니다.".to_string());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("계정 정보가 포함된 URL은 등록할 수 없습니다.".to_string());
    }

    Ok(url)
}

fn validate_target_url(value: &str) -> Result<(), String> {
    parse_target_url(value).map(|_| ())
}

/// 저장할 때는 해석한 주소를 그대로 직렬화해 표기를 통일합니다.
pub(crate) fn normalize_target_url(value: &str) -> Result<String, String> {
    Ok(parse_target_url(value)?.to_string())
}

/// CSS selector로 허용하는 문자입니다. 선택자 목록(`,`), 규칙 블록(`{}`), 이스케이프(`\`)와
/// 스크립트 실행으로 이어질 수 있는 표기는 받지 않습니다.
fn validate_css_locator(value: &str) -> Result<(), String> {
    const FORBIDDEN: [char; 9] = ['{', '}', ';', ',', '\\', '`', '@', '!', '\''];

    if value
        .chars()
        .any(|character| FORBIDDEN.contains(&character))
    {
        return Err("selector에 사용할 수 없는 문자가 있습니다.".to_string());
    }
    if !value.chars().all(|character| {
        character.is_ascii_alphanumeric() || "#._-[]=\":()*^$~+ >".contains(character)
    }) {
        return Err("selector에 사용할 수 없는 문자가 있습니다.".to_string());
    }

    let lowered = value.to_ascii_lowercase();
    if lowered.contains("javascript:") {
        return Err("selector에 사용할 수 없는 표기가 있습니다.".to_string());
    }
    if value.starts_with('>') || value.ends_with('>') || value.contains(">>") {
        return Err("selector의 결합 표기가 올바르지 않습니다.".to_string());
    }

    // 괄호는 우리가 만드는 nth-child()/nth-of-type()에만 허용합니다.
    // 그 밖의 함수형 선택자(:is, :has, :not 등)는 받지 않습니다.
    if value.matches('(').count() != value.matches(')').count() {
        return Err("selector의 괄호가 닫히지 않았습니다.".to_string());
    }
    let mut cursor = 0;
    while let Some(open_offset) = lowered[cursor..].find('(') {
        let open = cursor + open_offset;
        if !(lowered[..open].ends_with("nth-child") || lowered[..open].ends_with("nth-of-type")) {
            return Err("selector에 사용할 수 없는 함수형 표기가 있습니다.".to_string());
        }
        let Some(close_offset) = lowered[open..].find(')') else {
            return Err("selector의 괄호가 닫히지 않았습니다.".to_string());
        };
        let inner = &lowered[open + 1..open + close_offset];
        if inner.is_empty()
            || !inner.chars().all(|character| {
                character.is_ascii_digit() || matches!(character, 'n' | '+' | '-' | ' ')
            })
        {
            return Err("selector의 nth 표기가 올바르지 않습니다.".to_string());
        }
        cursor = open + close_offset + 1;
    }

    Ok(())
}

fn validate_locator(locator: &AutomationLocator) -> Result<(), String> {
    let value = locator.value.trim();
    if value.is_empty() || value.len() > MAX_LOCATOR_VALUE_LEN {
        return Err("화면 요소 위치 값은 1~300자로 입력하세요.".to_string());
    }
    validate_plain_field(value)?;

    match locator.kind {
        LocatorKind::Id => {
            if !value.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':')
            }) {
                return Err("요소 id에 사용할 수 없는 문자가 있습니다.".to_string());
            }
        }
        LocatorKind::Name => {
            if value.contains(['"', '\\', ']']) {
                return Err("name 값에 사용할 수 없는 문자가 있습니다.".to_string());
            }
        }
        LocatorKind::Label => {}
        LocatorKind::Css => validate_css_locator(value)?,
    }

    Ok(())
}

fn validate_header_text(value: &str, description: &str) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_FIELD_LABEL_LEN {
        return Err(format!("{description}은 1~80자로 입력하세요."));
    }
    validate_plain_field(trimmed)
}

fn validate_table(table: &AutomationTable) -> Result<(), String> {
    validate_locator(&table.locator)?;

    if table.identity_headers.is_empty() || table.identity_headers.len() > MAX_TABLE_HEADERS {
        return Err(format!(
            "행 식별 머리글은 1~{MAX_TABLE_HEADERS}개가 필요합니다."
        ));
    }
    if table.date_labels.is_empty() || table.date_labels.len() > MAX_TABLE_DATES {
        return Err(format!("날짜 열은 1~{MAX_TABLE_DATES}개가 필요합니다."));
    }
    for header in &table.identity_headers {
        validate_header_text(header, "행 식별 머리글")?;
    }
    for label in &table.date_labels {
        validate_header_text(label, "날짜 열 머리글")?;
    }

    Ok(())
}

fn validate_watch(watch: &AutomationWatch) -> Result<(), String> {
    if watch.list.kind != LocatorKind::Css {
        return Err("감시할 목록 위치는 css 선택자여야 합니다.".to_string());
    }
    validate_locator(&watch.list)?;
    validate_locator(&watch.identity)?;

    Ok(())
}

pub(crate) fn validate_recipe(recipe: &AutomationRecipe) -> Result<(), String> {
    let id = recipe.id.trim();
    let name = recipe.name.trim();

    if id.is_empty() || id.len() > MAX_RECIPE_ID_LEN {
        return Err("자동화 식별자가 올바르지 않습니다.".to_string());
    }
    if name.is_empty() || name.len() > MAX_RECIPE_NAME_LEN {
        return Err("버튼 이름은 1~80자로 입력하세요.".to_string());
    }

    validate_plain_field(id)?;
    validate_plain_field(name)?;
    validate_target_url(recipe.target_url.trim())?;

    match recipe.kind {
        AutomationKind::Shortcut => {
            if !recipe.fields.is_empty() || recipe.table.is_some() || recipe.watch.is_some() {
                return Err("바로가기에는 입력 필드를 둘 수 없습니다.".to_string());
            }
        }
        AutomationKind::Fill => {
            if recipe.table.is_some() {
                return Err("자동입력 레시피에는 표 정보를 둘 수 없습니다.".to_string());
            }
            if recipe.watch.is_some() {
                return Err("자동입력 레시피에는 감시 정보를 둘 수 없습니다.".to_string());
            }
            if recipe.fields.is_empty() || recipe.fields.len() > MAX_RECIPE_FIELDS {
                return Err(format!(
                    "자동입력 레시피는 필드 1~{MAX_RECIPE_FIELDS}개가 필요합니다."
                ));
            }
            for field in &recipe.fields {
                validate_header_text(&field.label, "필드 이름")?;
                validate_locator(&field.locator)?;
            }
        }
        AutomationKind::TableFill => {
            if !recipe.fields.is_empty() {
                return Err("표 입력 레시피에는 개별 필드를 둘 수 없습니다.".to_string());
            }
            if recipe.watch.is_some() {
                return Err("표 입력 레시피에는 감시 정보를 둘 수 없습니다.".to_string());
            }
            let table = recipe
                .table
                .as_ref()
                .ok_or_else(|| "표 입력 레시피에는 표 정보가 필요합니다.".to_string())?;
            validate_table(table)?;
        }
        AutomationKind::Watch => {
            if !recipe.fields.is_empty() {
                return Err("감시 레시피에는 입력 필드를 둘 수 없습니다.".to_string());
            }
            if recipe.table.is_some() {
                return Err("감시 레시피에는 표 정보를 둘 수 없습니다.".to_string());
            }
            let watch = recipe
                .watch
                .as_ref()
                .ok_or_else(|| "감시 레시피에는 목록 위치가 필요합니다.".to_string())?;
            validate_watch(watch)?;
        }
    }

    Ok(())
}

/// 저장 전에 값 표기를 통일하고 검증합니다. 앱 명령과 브리지가 같은 규칙을 씁니다.
pub(crate) fn normalize_recipe(mut recipe: AutomationRecipe) -> Result<AutomationRecipe, String> {
    recipe.id = recipe.id.trim().to_string();
    recipe.name = recipe.name.trim().to_string();
    recipe.target_url = normalize_target_url(recipe.target_url.trim())?;
    for field in &mut recipe.fields {
        field.label = field.label.trim().to_string();
        field.locator.value = field.locator.value.trim().to_string();
    }
    if let Some(table) = &mut recipe.table {
        table.locator.value = table.locator.value.trim().to_string();
        for header in &mut table.identity_headers {
            *header = header.trim().to_string();
        }
        for label in &mut table.date_labels {
            *label = label.trim().to_string();
        }
    }
    if let Some(watch) = &mut recipe.watch {
        watch.list.value = watch.list.value.trim().to_string();
        watch.identity.value = watch.identity.value.trim().to_string();
    }
    validate_recipe(&recipe)?;
    Ok(recipe)
}

pub(crate) fn automation_config_dir(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let directory = app_handle
        .path()
        .app_config_dir()
        .map_err(|error| format!("자동화 설정 경로를 확인하지 못했습니다: {error}"))?;

    fs::create_dir_all(&directory)
        .map_err(|error| format!("자동화 설정 디렉터리를 만들지 못했습니다: {error}"))?;

    Ok(directory)
}

pub(crate) fn automation_recipes_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(automation_config_dir(app_handle)?.join(AUTOMATION_RECIPES_FILE))
}

/// 저장된 레시피를 읽기만 합니다. 조회 경로는 원본을 덮어쓰지 않습니다.
pub(crate) fn read_automation_recipes(path: &Path) -> Result<Vec<AutomationRecipe>, String> {
    Ok(RecipeFile::read(path)?.recipes)
}

/// 바로가기는 4열, 자동입력은 필드 JSON을 담은 5열로 기록합니다.
/// 기존 4열 파일을 그대로 읽을 수 있도록 5열은 선택 사항입니다.
fn encode_recipe(recipe: &AutomationRecipe) -> Result<String, String> {
    let kind = match recipe.kind {
        AutomationKind::Shortcut => "shortcut",
        AutomationKind::Fill => "fill",
        AutomationKind::TableFill => "table_fill",
        AutomationKind::Watch => "watch",
    };
    let head = format!(
        "{}\t{kind}\t{}\t{}",
        recipe.id, recipe.name, recipe.target_url
    );
    if recipe.fields.is_empty() && recipe.table.is_none() && recipe.watch.is_none() {
        return Ok(head);
    }

    let payload = AutomationPayload {
        fields: recipe.fields.clone(),
        table: recipe.table.clone(),
        watch: recipe.watch.clone(),
    };
    let payload = serde_json::to_string(&payload)
        .map_err(|error| format!("자동화 설정을 저장하지 못했습니다: {error}"))?;
    Ok(format!("{head}\t{payload}"))
}

fn decode_recipe(line: &str) -> Result<AutomationRecipe, String> {
    let mut columns = line.splitn(5, '\t');
    // 저장·검증과 같은 기준을 쓰도록 읽을 때도 값을 trim합니다.
    let id = columns.next().unwrap_or("").trim();
    let kind = columns.next().unwrap_or("").trim();
    let name = columns.next().unwrap_or("").trim();
    let target_url = columns.next().unwrap_or("").trim();
    let payload = columns.next().unwrap_or("").trim();

    if id.is_empty() || name.is_empty() || target_url.is_empty() {
        return Err("자동화 설정 형식이 올바르지 않습니다.".to_string());
    }

    let kind = match kind {
        "shortcut" => AutomationKind::Shortcut,
        "fill" => AutomationKind::Fill,
        "table_fill" => AutomationKind::TableFill,
        "watch" => AutomationKind::Watch,
        _ => return Err("자동화 설정 형식이 올바르지 않습니다.".to_string()),
    };
    let payload = if payload.is_empty() {
        AutomationPayload::default()
    } else {
        // 이전 형식은 필드 배열만 저장했습니다.
        serde_json::from_str::<AutomationPayload>(payload)
            .or_else(|_| {
                serde_json::from_str::<Vec<AutomationField>>(payload).map(|fields| {
                    AutomationPayload {
                        fields,
                        table: None,
                        watch: None,
                    }
                })
            })
            .map_err(|_| "자동화 설정의 필드 형식이 올바르지 않습니다.".to_string())?
    };

    let recipe = AutomationRecipe {
        id: id.to_string(),
        kind,
        name: name.to_string(),
        target_url: target_url.to_string(),
        fields: payload.fields,
        table: payload.table,
        watch: payload.watch,
    };
    validate_recipe(&recipe)?;
    Ok(recipe)
}

fn decode_recipes(content: &str) -> (Vec<AutomationRecipe>, Vec<String>) {
    let mut recipes = Vec::new();
    let mut rejected = Vec::new();

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match decode_recipe(line) {
            Ok(recipe) => recipes.push(recipe),
            Err(_) => rejected.push(line.to_string()),
        }
    }

    (recipes, rejected)
}

fn quarantine_invalid_lines(directory: &Path, lines: &[String]) -> Result<(), String> {
    let path = directory.join(AUTOMATION_RECIPES_QUARANTINE_FILE);
    let mut content = match fs::read_to_string(&path) {
        Ok(existing) => existing,
        // 격리 파일이 아직 없을 때만 빈 파일로 시작합니다.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        // 그 밖의 읽기 오류는 기존 격리 내용을 보존하기 위해 그대로 실패시킵니다.
        Err(error) => return Err(format!("기존 격리 파일을 읽지 못했습니다: {error}")),
    };

    let added: usize = lines.iter().map(|line| line.len() + 1).sum();
    if content.len() + added > MAX_QUARANTINE_BYTES {
        return Err(format!(
            "격리 파일이 상한({MAX_QUARANTINE_BYTES} bytes)을 넘어 복구를 멈췄습니다. {AUTOMATION_RECIPES_QUARANTINE_FILE} 파일을 정리한 뒤 다시 시도하세요."
        ));
    }

    for line in lines {
        content.push_str(line);
        content.push('\n');
    }

    write_file_atomically(&path, &content)
}

/// 레시피 파일을 읽은 결과입니다.
///
/// `damaged_lines`가 비어 있지 않으면 원본에 손상된 행이 남아 있다는 뜻이며,
/// 격리에 성공하기 전까지 원본을 덮어쓰면 그 행이 사라집니다.
struct RecipeFile {
    recipes: Vec<AutomationRecipe>,
    damaged_lines: Vec<String>,
}

impl RecipeFile {
    /// 파일을 읽기만 하고 수정하지 않습니다.
    fn read(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self {
                recipes: Vec::new(),
                damaged_lines: Vec::new(),
            });
        }

        let content = fs::read_to_string(path)
            .map_err(|error| format!("자동화 설정을 읽지 못했습니다: {error}"))?;
        let (recipes, damaged_lines) = decode_recipes(&content);

        Ok(Self {
            recipes,
            damaged_lines,
        })
    }

    /// 손상된 행을 격리한 뒤에만 원본을 정리합니다.
    /// 격리에 실패하면 원본과 기존 격리 파일을 건드리지 않고 오류를 돌려줍니다.
    fn recover(&self, path: &Path) -> Result<(), String> {
        if self.damaged_lines.is_empty() {
            return Ok(());
        }

        let directory = path
            .parent()
            .ok_or_else(|| "자동화 설정 경로를 확인하지 못했습니다.".to_string())?;
        quarantine_invalid_lines(directory, &self.damaged_lines)?;
        write_automation_recipes(path, &self.recipes)
    }
}

/// 원본을 덮어쓰는 명령의 사전 조건입니다.
/// 손상된 행을 격리하지 못하면 오류를 돌려주고 원본을 건드리지 않습니다.
fn load_recipes_for_update(path: &Path) -> Result<Vec<AutomationRecipe>, String> {
    let file = RecipeFile::read(path)?;
    file.recover(path)?;
    Ok(file.recipes)
}

pub(crate) fn upsert_automation_recipe(
    path: &Path,
    recipe: AutomationRecipe,
) -> Result<Vec<AutomationRecipe>, String> {
    let mut recipes = load_recipes_for_update(path)?;

    match recipes.iter().position(|item| item.id == recipe.id) {
        Some(index) => recipes[index] = recipe,
        None if recipes.len() >= MAX_RECIPES => {
            return Err(format!(
                "업무 버튼은 최대 {MAX_RECIPES}개까지 등록할 수 있습니다."
            ));
        }
        None => recipes.push(recipe),
    }
    recipes.sort_by(|left, right| left.name.cmp(&right.name));

    write_automation_recipes(path, &recipes)?;
    Ok(recipes)
}

fn remove_automation_recipe(path: &Path, recipe_id: &str) -> Result<Vec<AutomationRecipe>, String> {
    let mut recipes = load_recipes_for_update(path)?;
    recipes.retain(|recipe| recipe.id != recipe_id);

    write_automation_recipes(path, &recipes)?;
    Ok(recipes)
}

fn write_automation_recipes(path: &Path, recipes: &[AutomationRecipe]) -> Result<(), String> {
    let mut lines = Vec::with_capacity(recipes.len());
    for recipe in recipes {
        lines.push(encode_recipe(recipe)?);
    }
    let mut content = lines.join("\n");
    if !content.is_empty() {
        content.push('\n');
    }

    write_file_atomically(path, &content)
}

fn write_temp_file(path: &Path, content: &[u8]) -> std::io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(content)?;
    file.sync_all()
}

pub(crate) fn write_file_atomically(path: &Path, content: &str) -> Result<(), String> {
    write_bytes_atomically(path, content.as_bytes())
}

pub(crate) fn write_bytes_atomically(path: &Path, content: &[u8]) -> Result<(), String> {
    let directory = path
        .parent()
        .ok_or_else(|| "자동화 설정 경로를 확인하지 못했습니다.".to_string())?;
    let file_name = path
        .file_name()
        .ok_or_else(|| "자동화 설정 경로를 확인하지 못했습니다.".to_string())?;
    let temp_path = directory.join(format!("{}.tmp", file_name.to_string_lossy()));

    if let Err(error) = write_temp_file(&temp_path, content) {
        let _ = fs::remove_file(&temp_path);
        return Err(format!("자동화 설정을 저장하지 못했습니다: {error}"));
    }

    restrict_file_permissions(&temp_path);

    if let Err(error) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(format!("자동화 설정을 저장하지 못했습니다: {error}"));
    }

    Ok(())
}

/// 레시피 파일에는 개인 업무 주소만 들어가지만, 다른 사용자 계정에 노출되지 않도록
/// 소유자 전용 권한으로 좁힙니다. Windows에서는 사용자 프로필 ACL을 그대로 따릅니다.
#[cfg(unix)]
fn restrict_file_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    if let Err(error) = fs::set_permissions(path, fs::Permissions::from_mode(0o600)) {
        eprintln!("자동화 설정 파일 권한을 좁히지 못했습니다: {error}");
    }
}

#[cfg(not(unix))]
fn restrict_file_permissions(_path: &Path) {}

/// 감시 레시피별로 마지막으로 본 식별자와 확인 시각만 보관합니다.
///
/// 학생 이름·신청 사유 같은 본문은 저장하지 않습니다.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WatchStateEntry {
    pub(crate) recipe_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) identifiers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) checked_at_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub(crate) paused: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) new_identifiers: Vec<String>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WatchScanOutcome {
    pub(crate) recipe_id: String,
    pub(crate) paused: bool,
    pub(crate) total: usize,
    pub(crate) new_identifiers: Vec<String>,
    pub(crate) checked_at_ms: u64,
}

pub(crate) fn automation_watch_state_path(
    app_handle: &tauri::AppHandle,
) -> Result<PathBuf, String> {
    Ok(automation_config_dir(app_handle)?.join(AUTOMATION_WATCH_STATE_FILE))
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

/// 감시 상태는 비교용 파생 데이터입니다. 형식이 깨졌으면 격리해 보관하고
/// 빈 상태로 다시 시작합니다(다음 스캔에서 한 번 다시 알릴 수 있습니다).
pub(crate) fn read_watch_state(path: &Path) -> Result<Vec<WatchStateEntry>, String> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("감시 상태를 읽지 못했습니다: {error}")),
    };
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }
    if content.len() > MAX_WATCH_STATE_BYTES {
        return Err("감시 상태 파일이 너무 큽니다. 파일을 정리한 뒤 다시 시도하세요.".to_string());
    }

    match serde_json::from_str::<Vec<WatchStateEntry>>(&content) {
        Ok(entries) => Ok(entries),
        Err(error) => {
            eprintln!("감시 상태 형식이 올바르지 않아 격리합니다: {error}");
            if let Some(directory) = path.parent() {
                let quarantine = directory.join(AUTOMATION_WATCH_STATE_QUARANTINE_FILE);
                let _ = fs::rename(path, &quarantine);
            }
            Ok(Vec::new())
        }
    }
}

fn write_watch_state(path: &Path, entries: &[WatchStateEntry]) -> Result<(), String> {
    let content = if entries.is_empty() {
        String::new()
    } else {
        let mut content = serde_json::to_string_pretty(entries)
            .map_err(|error| format!("감시 상태를 저장하지 못했습니다: {error}"))?;
        content.push('\n');
        content
    };

    if content.len() > MAX_WATCH_STATE_BYTES {
        return Err("감시 상태가 너무 커져 저장하지 않았습니다.".to_string());
    }
    write_file_atomically(path, &content)
}

/// 식별자 목록을 다듬고 중복을 제거합니다. 값 자체는 저장 전에 검증합니다.
fn normalize_identifiers(values: Vec<String>) -> Result<Vec<String>, String> {
    if values.len() > MAX_WATCH_IDENTIFIERS {
        return Err(format!(
            "한 번에 저장할 수 있는 식별자는 {MAX_WATCH_IDENTIFIERS}개까지입니다."
        ));
    }

    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.chars().count() > MAX_WATCH_IDENTIFIER_LEN {
            return Err("감시 식별자가 너무 깁니다.".to_string());
        }
        validate_plain_field(trimmed)?;
        if seen.insert(trimmed.to_string()) {
            normalized.push(trimmed.to_string());
        }
    }

    Ok(normalized)
}

/// 스캔 결과를 저장하고 새로 생긴 식별자를 돌려줍니다.
///
/// 감시가 중지된 레시피는 상태를 바꾸지 않고 그대로 둡니다.
pub(crate) fn record_watch_scan(
    path: &Path,
    recipe_id: &str,
    identifiers: Vec<String>,
) -> Result<WatchScanOutcome, String> {
    let identifiers = normalize_identifiers(identifiers)?;
    let mut entries = read_watch_state(path)?;
    let index = entries
        .iter()
        .position(|entry| entry.recipe_id == recipe_id);
    let mut entry = match index {
        Some(index) => entries[index].clone(),
        None => WatchStateEntry {
            recipe_id: recipe_id.to_string(),
            ..WatchStateEntry::default()
        },
    };

    if entry.paused {
        return Ok(WatchScanOutcome {
            recipe_id: recipe_id.to_string(),
            paused: true,
            total: identifiers.len(),
            new_identifiers: Vec::new(),
            checked_at_ms: entry.checked_at_ms.unwrap_or_default(),
        });
    }

    let previous: HashSet<&String> = entry.identifiers.iter().collect();
    let new_identifiers: Vec<String> = identifiers
        .iter()
        .filter(|identifier| !previous.contains(identifier))
        .cloned()
        .collect();
    let checked_at_ms = now_millis();

    entry.identifiers = identifiers.clone();
    entry.checked_at_ms = Some(checked_at_ms);
    entry.new_identifiers = new_identifiers.clone();

    match index {
        Some(index) => entries[index] = entry,
        None => entries.push(entry),
    }
    entries.sort_by(|left, right| left.recipe_id.cmp(&right.recipe_id));
    write_watch_state(path, &entries)?;

    Ok(WatchScanOutcome {
        recipe_id: recipe_id.to_string(),
        paused: false,
        total: identifiers.len(),
        new_identifiers,
        checked_at_ms,
    })
}

fn update_watch_entry(
    path: &Path,
    recipe_id: &str,
    update: impl FnOnce(&mut WatchStateEntry),
) -> Result<Vec<WatchStateEntry>, String> {
    let mut entries = read_watch_state(path)?;
    if let Some(entry) = entries
        .iter_mut()
        .find(|entry| entry.recipe_id == recipe_id)
    {
        update(entry);
    }
    write_watch_state(path, &entries)?;
    Ok(entries)
}

/// 새 항목 알림을 확인 처리합니다. 이미 본 식별자는 다시 알리지 않습니다.
fn acknowledge_watch(path: &Path, recipe_id: &str) -> Result<Vec<WatchStateEntry>, String> {
    update_watch_entry(path, recipe_id, |entry| entry.new_identifiers.clear())
}

fn set_watch_paused(
    path: &Path,
    recipe_id: &str,
    paused: bool,
) -> Result<Vec<WatchStateEntry>, String> {
    update_watch_entry(path, recipe_id, |entry| entry.paused = paused)
}

/// 레시피를 지우면 감시 상태도 함께 정리합니다.
fn prune_watch_state(path: &Path, recipe_id: &str) -> Result<(), String> {
    let mut entries = read_watch_state(path)?;
    let before = entries.len();
    entries.retain(|entry| entry.recipe_id != recipe_id);
    if entries.len() == before {
        return Ok(());
    }
    write_watch_state(path, &entries)
}

/// 감시 상태 명령이 공통으로 확인하는 레시피 식별자입니다.
fn validate_recipe_id(recipe_id: &str) -> Result<String, String> {
    let recipe_id = recipe_id.trim();
    if recipe_id.is_empty() || recipe_id.len() > MAX_RECIPE_ID_LEN {
        return Err("자동화 식별자가 올바르지 않습니다.".to_string());
    }
    validate_plain_field(recipe_id)?;
    Ok(recipe_id.to_string())
}

/// 감시 레시피가 실제로 있을 때만 상태를 다룹니다.
fn ensure_watch_recipe(path: &Path, recipe_id: &str) -> Result<(), String> {
    let file = RecipeFile::read(path)?;
    let recipe = file
        .recipes
        .iter()
        .find(|recipe| recipe.id == recipe_id)
        .ok_or_else(|| "등록된 감시 레시피를 찾지 못했습니다.".to_string())?;
    if recipe.kind != AutomationKind::Watch {
        return Err("감시 레시피가 아닙니다.".to_string());
    }
    Ok(())
}

#[tauri::command]
fn list_automation_recipes(app_handle: tauri::AppHandle) -> Result<Vec<AutomationRecipe>, String> {
    let path = automation_recipes_path(&app_handle)?;
    let file = RecipeFile::read(&path)?;

    // 조회는 파일을 덮어쓰지 않으므로 격리 실패가 목록 표시를 막지 않습니다.
    // 손상된 행은 원본에 남고 다음 조회에서 복구를 다시 시도합니다.
    // 원본을 덮어쓰는 등록/삭제는 격리 성공을 사전 조건으로 요구합니다.
    if let Err(error) = file.recover(&path) {
        eprintln!("자동화 설정 복구를 미뤘습니다: {error}");
    }

    Ok(file.recipes)
}

#[tauri::command]
fn save_automation_recipe(
    app_handle: tauri::AppHandle,
    recipe: AutomationRecipe,
) -> Result<Vec<AutomationRecipe>, String> {
    let recipe = normalize_recipe(recipe)?;
    let path = automation_recipes_path(&app_handle)?;
    upsert_automation_recipe(&path, recipe)
}

#[tauri::command]
fn delete_automation_recipe(
    app_handle: tauri::AppHandle,
    recipe_id: String,
) -> Result<Vec<AutomationRecipe>, String> {
    let recipe_id = recipe_id.trim();
    if recipe_id.is_empty() || recipe_id.len() > MAX_RECIPE_ID_LEN {
        return Err("자동화 식별자가 올바르지 않습니다.".to_string());
    }
    validate_plain_field(recipe_id)?;

    let path = automation_recipes_path(&app_handle)?;
    let recipes = remove_automation_recipe(&path, recipe_id)?;

    // 레시피가 사라지면 감시 상태도 남기지 않습니다.
    if let Ok(state_path) = automation_watch_state_path(&app_handle)
        && let Err(error) = prune_watch_state(&state_path, recipe_id)
    {
        eprintln!("감시 상태를 정리하지 못했습니다: {error}");
    }

    Ok(recipes)
}

#[tauri::command]
fn list_automation_watch_state(
    app_handle: tauri::AppHandle,
) -> Result<Vec<WatchStateEntry>, String> {
    let path = automation_watch_state_path(&app_handle)?;
    read_watch_state(&path)
}

/// 확장이 스캔한 식별자 목록을 받아 새 항목만 골라 알림 상태로 남깁니다.
#[tauri::command]
fn record_automation_watch_scan(
    app_handle: tauri::AppHandle,
    recipe_id: String,
    identifiers: Vec<String>,
) -> Result<WatchScanOutcome, String> {
    let recipe_id = validate_recipe_id(&recipe_id)?;
    let recipes_path = automation_recipes_path(&app_handle)?;
    ensure_watch_recipe(&recipes_path, &recipe_id)?;

    let state_path = automation_watch_state_path(&app_handle)?;
    record_watch_scan(&state_path, &recipe_id, identifiers)
}

#[tauri::command]
fn acknowledge_automation_watch(
    app_handle: tauri::AppHandle,
    recipe_id: String,
) -> Result<Vec<WatchStateEntry>, String> {
    let recipe_id = validate_recipe_id(&recipe_id)?;
    let state_path = automation_watch_state_path(&app_handle)?;
    acknowledge_watch(&state_path, &recipe_id)
}

#[tauri::command]
fn set_automation_watch_paused(
    app_handle: tauri::AppHandle,
    recipe_id: String,
    paused: bool,
) -> Result<Vec<WatchStateEntry>, String> {
    let recipe_id = validate_recipe_id(&recipe_id)?;
    let recipes_path = automation_recipes_path(&app_handle)?;
    ensure_watch_recipe(&recipes_path, &recipe_id)?;

    let state_path = automation_watch_state_path(&app_handle)?;
    set_watch_paused(&state_path, &recipe_id, paused)
}

/// 브라우저 열기는 사용자의 응답을 기다리지 않도록 분리 실행하고,
/// 종료 상태만 백그라운드에서 수거합니다.
fn spawn_detached(program: &str, args: &[&str]) -> Result<(), String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("브라우저를 열지 못했습니다: {error}"))?;

    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_external_url(url: &str) -> Result<(), String> {
    spawn_detached("rundll32", &["url.dll,FileProtocolHandler", url])
}

#[cfg(target_os = "macos")]
fn launch_external_url(url: &str) -> Result<(), String> {
    spawn_detached("open", &[url])
}

#[cfg(target_os = "linux")]
fn launch_external_url(url: &str) -> Result<(), String> {
    spawn_detached("xdg-open", &[url])
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn launch_external_url(_url: &str) -> Result<(), String> {
    Err("현재 플랫폼에서는 외부 브라우저 열기를 지원하지 않습니다.".to_string())
}

/// 렌더러가 임의 URL을 열지 못하게, 저장된 레시피 id로만 실행합니다.
#[tauri::command]
fn open_automation_recipe(app_handle: tauri::AppHandle, recipe_id: String) -> Result<(), String> {
    let recipe_id = recipe_id.trim();
    if recipe_id.is_empty() || recipe_id.len() > MAX_RECIPE_ID_LEN {
        return Err("자동화 식별자가 올바르지 않습니다.".to_string());
    }
    validate_plain_field(recipe_id)?;

    let path = automation_recipes_path(&app_handle)?;
    let target_url = resolve_recipe_target(&path, recipe_id)?;
    launch_external_url(&target_url)
}

/// 저장된 레시피에서 실행할 주소를 찾습니다. 손상된 행 복구에 실패해도
/// 남아 있는 정상 레시피는 실행할 수 있어야 하므로 파일을 읽기만 합니다.
fn resolve_recipe_target(path: &Path, recipe_id: &str) -> Result<String, String> {
    let file = RecipeFile::read(path)?;
    let recipe = file
        .recipes
        .into_iter()
        .find(|recipe| recipe.id == recipe_id)
        .ok_or_else(|| "등록된 업무 버튼을 찾지 못했습니다.".to_string())?;

    // 파일이 외부에서 수정될 수 있으므로 실행 직전에 다시 검증합니다.
    validate_target_url(&recipe.target_url)?;
    Ok(recipe.target_url)
}

/// Session handed to the OS credential store.
///
/// Tokens never touch the recipe files, and nothing here is logged. The store
/// is Windows-first: other platforms report it as unavailable instead of
/// writing a token to a plain file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoredAuthSession {
    pub(crate) access_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) refresh_token: Option<String>,
    pub(crate) user_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) expires_at_ms: Option<u64>,
}

/// Windows Credential Manager rejects blobs larger than 2560 bytes, so the
/// stored JSON stays well below that.
const MAX_SESSION_BYTES: usize = 2_048;
// The credential-store entry only exists on platforms with a store.
#[cfg(windows)]
const SESSION_SERVICE: &str = "kr.schoolcollect.app";
#[cfg(windows)]
const SESSION_ACCOUNT: &str = "auth-session";

fn validate_auth_session(session: &StoredAuthSession) -> Result<String, String> {
    if session.access_token.trim().is_empty() {
        return Err("access token이 비어 있습니다.".to_string());
    }
    // The owner of local drafts is the token's subject, never a value the
    // renderer chooses. Refuse a session whose user id disagrees with it.
    let subject = token_subject(&session.access_token)?;
    if session.user_id.trim() != subject {
        return Err("세션 사용자와 토큰의 사용자가 다릅니다.".to_string());
    }
    for value in [
        Some(session.access_token.as_str()),
        session.refresh_token.as_deref(),
        Some(session.user_id.as_str()),
        session.email.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        validate_plain_field(value)?;
    }

    let encoded = serde_json::to_string(session)
        .map_err(|error| format!("세션을 저장할 수 없습니다: {error}"))?;
    if encoded.len() > MAX_SESSION_BYTES {
        return Err("세션이 보안 저장소 크기 제한을 넘습니다.".to_string());
    }
    Ok(encoded)
}

/// Reads the `sub` claim of an identity-provider access token.
///
/// The signature is not checked here: the API does that for every request.
/// Locally the subject only decides which drafts on this computer belong to
/// the signed-in person, so a forged token could claim nothing it does not
/// already hold, and it grants no server access.
fn token_subject(access_token: &str) -> Result<String, String> {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

    let mut parts = access_token.trim().split('.');
    let (Some(_header), Some(payload), Some(_signature), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err("access token 형식이 올바르지 않습니다.".to_string());
    };
    let bytes = URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .map_err(|_| "access token을 읽지 못했습니다.".to_string())?;
    let claims: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|_| "access token을 읽지 못했습니다.".to_string())?;
    match claims.get("sub").and_then(serde_json::Value::as_str) {
        Some(subject) if !subject.trim().is_empty() && subject.len() <= 256 => {
            Ok(subject.trim().to_string())
        }
        _ => Err("access token에 사용자 정보가 없습니다.".to_string()),
    }
}

#[cfg(windows)]
fn auth_session_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(SESSION_SERVICE, SESSION_ACCOUNT)
        .map_err(|error| format!("보안 저장소를 열지 못했습니다: {error}"))
}

/// Saves the session in the OS credential store.
#[tauri::command]
fn save_auth_session(session: StoredAuthSession) -> Result<(), String> {
    let encoded = validate_auth_session(&session)?;

    #[cfg(windows)]
    {
        auth_session_entry()?
            .set_password(&encoded)
            .map_err(|error| format!("세션을 보안 저장소에 저장하지 못했습니다: {error}"))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = encoded;
        Err("이 플랫폼에서는 아직 보안 저장소를 지원하지 않습니다.".to_string())
    }
}

/// Reads the stored session. A missing entry is `None`, not an error.
#[tauri::command]
fn load_auth_session() -> Result<Option<StoredAuthSession>, String> {
    #[cfg(windows)]
    {
        match auth_session_entry()?.get_password() {
            Ok(encoded) => serde_json::from_str::<StoredAuthSession>(&encoded)
                .map(Some)
                .map_err(|_| "저장된 세션을 읽지 못했습니다.".to_string()),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(format!("세션을 읽지 못했습니다: {error}")),
        }
    }
    #[cfg(not(windows))]
    {
        Ok(None)
    }
}

/// Removes the stored session. Signing out must not leave a token behind.
#[tauri::command]
fn clear_auth_session() -> Result<(), String> {
    #[cfg(windows)]
    {
        match auth_session_entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(format!("세션을 지우지 못했습니다: {error}")),
        }
    }
    #[cfg(not(windows))]
    {
        Ok(())
    }
}

/// How long the app waits for the identity provider to redirect back before it
/// gives up and closes the loopback listener.
const BROWSER_LOGIN_TIMEOUT: Duration = Duration::from_secs(180);
/// The public anon key is short; anything longer is a mistake, not a header.
const MAX_ANON_KEY_LEN: usize = 4_096;

/// One in-flight browser login attempt. The sender ends the wait (and closes
/// the loopback listener); it lives here so `cancel_browser_login` can reach it.
#[derive(Default)]
struct BrowserLoginState(Mutex<Option<oneshot::Sender<()>>>);

/// The identity provider base URL is an outbound target like any other, so it
/// goes through the same `parse_target_url` rules that `validate_target_url`
/// exposes (http/https, a host, no embedded credentials, bounded length), plus
/// the https requirement: plain http is allowed only for a loopback test
/// provider.
fn validate_identity_base_url(value: &str) -> Result<Url, String> {
    let mut url = parse_target_url(value)?;
    if url.scheme() != "https" && url.host_str() != Some("127.0.0.1") {
        return Err(
            "identity provider 주소는 https여야 합니다(테스트용 loopback만 http 허용).".to_string(),
        );
    }
    // The provider paths are resolved against this base (`authorize`, `token`),
    // so a missing trailing slash would silently drop the last segment.
    if !url.path().ends_with('/') {
        let path = format!("{}/", url.path());
        url.set_path(&path);
    }
    Ok(url)
}

/// The anon key is public by design, but it becomes a request header, so it
/// must still look like one value.
fn validate_anon_key(value: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.len() > MAX_ANON_KEY_LEN {
        return Err("공개 anon key가 올바르지 않습니다.".to_string());
    }
    if value.chars().any(char::is_whitespace) {
        return Err("공개 anon key에는 공백 문자를 쓸 수 없습니다.".to_string());
    }
    Ok(())
}

/// Signs in through the identity provider in the system browser (PKCE).
///
/// The renderer passes only public values: the provider base URL, the
/// publishable anon key and the provider name. No password is accepted, and
/// the PKCE verifier never leaves this process.
#[tauri::command]
async fn start_browser_login(
    state: tauri::State<'_, BrowserLoginState>,
    base_url: String,
    anon_key: String,
    provider: String,
) -> Result<browser_login::BrowserLoginSession, browser_login::LoginFailure> {
    let base_url = validate_identity_base_url(base_url.trim())
        .map_err(|message| browser_login::LoginFailure::invalid_config(&message))?;
    validate_anon_key(&anon_key)
        .map_err(|message| browser_login::LoginFailure::invalid_config(&message))?;
    let provider = provider.trim().to_string();

    // One attempt at a time. The sender is how the cancel command ends this
    // attempt, and it is taken back out as soon as the attempt is over.
    let (cancel, cancelled) = oneshot::channel();
    {
        let mut attempt = state.0.lock().map_err(|_| {
            browser_login::LoginFailure::invalid_config("로그인 상태를 확인하지 못했습니다.")
        })?;
        if attempt.is_some() {
            return Err(browser_login::LoginFailure::already_running());
        }
        *attempt = Some(cancel);
    }

    let result = browser_login::run_browser_login(
        &base_url,
        &provider,
        &anon_key,
        BROWSER_LOGIN_TIMEOUT,
        |authorize_url| launch_external_url(authorize_url.as_str()),
        cancelled,
    )
    .await;

    if let Ok(mut attempt) = state.0.lock() {
        *attempt = None;
    }
    result.map_err(browser_login::LoginFailure::from)
}

/// Stops an in-flight browser login. Cancelling closes the loopback listener,
/// so a late callback finds nothing to talk to. Returns whether an attempt was
/// actually cancelled.
#[tauri::command]
fn cancel_browser_login(state: tauri::State<'_, BrowserLoginState>) -> bool {
    let Ok(mut attempt) = state.0.lock() else {
        return false;
    };
    match attempt.take() {
        Some(cancel) => cancel.send(()).is_ok(),
        None => false,
    }
}

/// Local draft database in the app data directory.
fn drafts_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let directory = app_handle
        .path()
        .app_data_dir()
        .map_err(|error| format!("초안 저장 경로를 확인하지 못했습니다: {error}"))?;
    Ok(directory.join(drafts::DRAFTS_FILE))
}

/// 저장된 세션의 사용자와 요청한 사용자가 같은지 판정합니다.
///
/// 렌더러는 신뢰 경계 밖이므로, 초안 명령은 지금 저장된 세션의 사용자에
/// 대해서만 동작해야 합니다.
#[cfg(any(windows, test))]
fn session_user_matches(stored: Option<&StoredAuthSession>, requested: &str) -> Result<(), String> {
    let requested = requested.trim();
    if requested.is_empty() {
        return Err("초안 사용자 식별자가 올바르지 않습니다.".to_string());
    }
    let Some(session) = stored else {
        return Err("로그인 세션이 없어 이 컴퓨터의 초안을 사용할 수 없습니다.".to_string());
    };
    // Compare against the token's subject so an edited credential entry
    // cannot redirect drafts to another person.
    let owner = token_subject(&session.access_token)?;
    if owner != requested {
        return Err("지금 로그인한 사용자의 초안만 사용할 수 있습니다.".to_string());
    }
    Ok(())
}

/// 저장된 세션과 대조합니다. 다른 플랫폼에는 아직 세션 저장소가 없습니다.
#[cfg(windows)]
fn require_session_user(requested: &str) -> Result<(), String> {
    let stored = load_auth_session()?;
    session_user_matches(stored.as_ref(), requested)
}

/// 세션 저장소가 없는 플랫폼에서는 대조할 기준이 없어 지금은 통과시킵니다.
#[cfg(not(windows))]
fn require_session_user(_requested: &str) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
fn save_local_draft(
    app_handle: tauri::AppHandle,
    draft: drafts::LocalDraft,
) -> Result<drafts::LocalDraft, String> {
    require_session_user(&draft.user_id)?;
    let path = drafts_path(&app_handle)?;
    drafts::save_draft(&path, &draft, now_millis())
}

#[tauri::command]
fn list_local_drafts(
    app_handle: tauri::AppHandle,
    tenant_id: String,
    user_id: String,
) -> Result<Vec<drafts::LocalDraft>, String> {
    require_session_user(&user_id)?;
    let path = drafts_path(&app_handle)?;
    drafts::list_drafts(&path, tenant_id.trim(), user_id.trim())
}

#[tauri::command]
fn delete_local_draft(
    app_handle: tauri::AppHandle,
    tenant_id: String,
    user_id: String,
    collect_id: String,
) -> Result<(), String> {
    require_session_user(&user_id)?;
    let path = drafts_path(&app_handle)?;
    drafts::delete_draft(&path, tenant_id.trim(), user_id.trim(), collect_id.trim())
}

/// One draft, used when a collect screen opens after a restart.
#[tauri::command]
fn load_local_draft(
    app_handle: tauri::AppHandle,
    tenant_id: String,
    user_id: String,
    collect_id: String,
) -> Result<Option<drafts::LocalDraft>, String> {
    require_session_user(&user_id)?;
    let path = drafts_path(&app_handle)?;
    drafts::find_draft(&path, tenant_id.trim(), user_id.trim(), collect_id.trim())
}

/// Signing out clears the drafts of that user so a shared computer keeps no work.
#[tauri::command]
fn clear_local_drafts(app_handle: tauri::AppHandle, user_id: String) -> Result<usize, String> {
    require_session_user(&user_id)?;
    let path = drafts_path(&app_handle)?;
    drafts::clear_user_drafts(&path, user_id.trim())
}

/// Largest export the desktop app writes to disk.
const MAX_EXPORT_BYTES: usize = 4 * 1024 * 1024;
/// Largest attachment the server accepts, so the largest one saved locally.
const MAX_SAVED_ATTACHMENT_BYTES: usize = 10 * 1024 * 1024;

/// Keeps only a file name, never a path, so an export cannot escape the folder.
fn sanitize_export_name(file_name: &str) -> Result<String, String> {
    let trimmed = file_name.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 120 {
        return Err("파일 이름이 올바르지 않습니다.".to_string());
    }
    if trimmed.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|'])
        || trimmed.contains("..")
        || trimmed.starts_with('.')
    {
        return Err("파일 이름에 사용할 수 없는 문자가 있습니다.".to_string());
    }
    if trimmed.contains(['\t', '\r', '\n']) {
        return Err("파일 이름에 사용할 수 없는 문자가 있습니다.".to_string());
    }
    Ok(trimmed.to_string())
}

/// Writes an exported file into the user's Downloads folder.
///
/// The renderer sends text it already received from the API; the app never
/// invents the content. An existing file is never overwritten: a numeric suffix
/// is added instead.
#[tauri::command]
fn save_export_file(
    app_handle: tauri::AppHandle,
    file_name: String,
    content: String,
) -> Result<String, String> {
    if content.len() > MAX_EXPORT_BYTES {
        return Err("내보낼 내용이 너무 큽니다.".to_string());
    }
    save_to_downloads(&app_handle, &file_name, content.as_bytes())
}

/// Saves a downloaded attachment into the user's Downloads folder, with the
/// same name rules and no-overwrite behaviour as exports.
#[tauri::command]
fn save_downloaded_file(
    app_handle: tauri::AppHandle,
    file_name: String,
    content: Vec<u8>,
) -> Result<String, String> {
    if content.is_empty() || content.len() > MAX_SAVED_ATTACHMENT_BYTES {
        return Err("저장할 파일 크기가 올바르지 않습니다.".to_string());
    }
    save_to_downloads(&app_handle, &file_name, &content)
}

fn save_to_downloads(
    app_handle: &tauri::AppHandle,
    file_name: &str,
    content: &[u8],
) -> Result<String, String> {
    let file_name = sanitize_export_name(file_name)?;
    let directory = app_handle
        .path()
        .download_dir()
        .map_err(|error| format!("다운로드 폴더를 찾지 못했습니다: {error}"))?;
    fs::create_dir_all(&directory)
        .map_err(|error| format!("다운로드 폴더를 만들지 못했습니다: {error}"))?;
    write_new_file_in(&directory, &file_name, content)
        .map(|path| path.to_string_lossy().to_string())
}

/// Writes `content` to a file that did not exist, trying `name`, then
/// `stem (1).ext`, `stem (2).ext`, ... The existence check and the creation
/// are one step (`create_new`), so two saves at once can never take the same
/// name and an existing file is never overwritten or truncated.
fn write_new_file_in(directory: &Path, file_name: &str, content: &[u8]) -> Result<PathBuf, String> {
    let (stem, extension) = match file_name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => (stem.to_string(), format!(".{extension}")),
        _ => (file_name.to_string(), String::new()),
    };

    for counter in 0..=1_000 {
        let candidate = if counter == 0 {
            directory.join(file_name)
        } else {
            directory.join(format!("{stem} ({counter}){extension}"))
        };
        let mut file = match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("파일을 저장하지 못했습니다: {error}")),
        };
        let written = file.write_all(content).and_then(|()| file.sync_all());
        if let Err(error) = written {
            // A partial file under the chosen name must not be left behind.
            drop(file);
            let _ = fs::remove_file(&candidate);
            return Err(format!("파일을 저장하지 못했습니다: {error}"));
        }
        return Ok(candidate);
    }
    Err("같은 이름의 파일이 너무 많습니다.".to_string())
}

#[tauri::command]
fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BridgeInfo {
    port: u16,
    token: String,
    extension_id: String,
}

/// Pairing values the 자동화 화면 shows so the extension can be connected.
#[tauri::command]
fn automation_bridge_info(app_handle: tauri::AppHandle) -> Result<BridgeInfo, String> {
    Ok(BridgeInfo {
        port: bridge::BRIDGE_PORT,
        token: bridge::ensure_token(&app_handle)?,
        extension_id: bridge::ALLOWED_EXTENSION_ID.to_owned(),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(BrowserLoginState::default())
        .setup(|app| {
            // The bridge is a convenience for the browser extension; a failure
            // must not stop the desktop app from starting.
            if let Err(error) = bridge::spawn(app.handle()) {
                eprintln!("로컬 브리지를 시작하지 못했습니다: {error}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_version,
            automation_bridge_info,
            list_automation_recipes,
            save_automation_recipe,
            delete_automation_recipe,
            open_automation_recipe,
            list_automation_watch_state,
            record_automation_watch_scan,
            acknowledge_automation_watch,
            set_automation_watch_paused,
            save_auth_session,
            load_auth_session,
            clear_auth_session,
            start_browser_login,
            cancel_browser_login,
            save_local_draft,
            list_local_drafts,
            load_local_draft,
            delete_local_draft,
            clear_local_drafts,
            save_export_file,
            save_downloaded_file
        ])
        .run(tauri::generate_context!())
        .expect("failed to run School Collect");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_https_target_without_credentials() {
        assert!(validate_target_url("https://example.invalid/portal#draft").is_ok());
    }

    #[test]
    fn stored_session_must_fit_the_credential_store() {
        let session = StoredAuthSession {
            access_token: test_token("user-1"),
            refresh_token: Some("r".repeat(32)),
            user_id: "user-1".to_string(),
            email: Some("teacher@example.test".to_string()),
            expires_at_ms: Some(1_790_000_000_000),
        };
        assert!(validate_auth_session(&session).is_ok());

        let mut empty = session.clone();
        empty.access_token = "   ".to_string();
        assert!(validate_auth_session(&empty).is_err());

        let mut oversized = session.clone();
        oversized.refresh_token = Some("r".repeat(MAX_SESSION_BYTES + 1));
        assert!(validate_auth_session(&oversized).is_err());

        let mut broken = session.clone();
        broken.user_id = "user\t1".to_string();
        assert!(validate_auth_session(&broken).is_err());

        // The renderer cannot pick whose session this is: the id must be the
        // token's own subject, and a token without one is refused.
        let mut other_user = session.clone();
        other_user.user_id = "user-2".to_string();
        assert!(validate_auth_session(&other_user).is_err());

        let mut opaque = session.clone();
        opaque.access_token = "a".repeat(64);
        assert!(validate_auth_session(&opaque).is_err());
    }

    #[test]
    fn export_file_names_stay_inside_the_downloads_folder() {
        assert_eq!(
            sanitize_export_name("collect-2026-09-27.csv").unwrap(),
            "collect-2026-09-27.csv"
        );
        assert_eq!(
            sanitize_export_name("결과 내보내기.csv").unwrap(),
            "결과 내보내기.csv"
        );

        for rejected in [
            "",
            "   ",
            "../escape.csv",
            "..\\escape.csv",
            "folder/file.csv",
            "folder\\file.csv",
            ".hidden.csv",
            "line\nbreak.csv",
            &"a".repeat(121),
        ] {
            assert!(
                sanitize_export_name(rejected).is_err(),
                "{rejected} must be rejected"
            );
        }
    }

    #[test]
    fn stored_session_round_trips_through_json() {
        let session = StoredAuthSession {
            access_token: test_token("user-1"),
            refresh_token: Some("r".repeat(32)),
            user_id: "user-1".to_string(),
            email: Some("teacher@example.test".to_string()),
            expires_at_ms: Some(1_790_000_000_000),
        };
        let encoded = serde_json::to_string(&session).expect("encode");
        let decoded: StoredAuthSession = serde_json::from_str(&encoded).expect("decode");
        assert_eq!(decoded, session);
    }

    #[test]
    fn drafts_are_bound_to_the_stored_session_user() {
        let session = StoredAuthSession {
            access_token: test_token("user-1"),
            refresh_token: None,
            user_id: "user-1".to_string(),
            email: None,
            expires_at_ms: None,
        };

        assert!(session_user_matches(Some(&session), "user-1").is_ok());
        assert!(session_user_matches(Some(&session), "  user-1  ").is_ok());

        // A renderer that asks for another user, an empty id, or with no stored
        // session must be refused before it reaches the local draft table.
        assert!(session_user_matches(Some(&session), "user-2").is_err());
        assert!(session_user_matches(Some(&session), "  ").is_err());
        assert!(session_user_matches(None, "user-1").is_err());

        // Editing the stored user id does not move ownership: the token's
        // subject decides.
        let mut edited = session.clone();
        edited.user_id = "user-2".to_string();
        assert!(session_user_matches(Some(&edited), "user-2").is_err());
        assert!(session_user_matches(Some(&edited), "user-1").is_ok());
    }

    #[test]
    fn token_subject_reads_only_well_formed_tokens() {
        assert_eq!(token_subject(&test_token("abc-123")).unwrap(), "abc-123");
        assert!(token_subject("not-a-jwt").is_err());
        assert!(token_subject("a.b").is_err());
        assert!(token_subject("a.b.c.d").is_err());
        assert!(token_subject("e30.e30.sig").is_err()); // `{}` has no subject
    }

    #[test]
    fn saving_never_overwrites_and_parallel_saves_get_distinct_names() {
        let directory = std::env::temp_dir().join(format!(
            "sc-save-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();

        // An existing file and a leftover `.tmp` sibling both stay untouched.
        fs::write(directory.join("계획서.pdf"), b"original").unwrap();
        fs::write(directory.join("계획서.pdf.tmp"), b"someone else's temp").unwrap();
        let saved = write_new_file_in(&directory, "계획서.pdf", b"new").unwrap();
        assert_eq!(saved.file_name().unwrap(), "계획서 (1).pdf");
        assert_eq!(fs::read(directory.join("계획서.pdf")).unwrap(), b"original");
        assert_eq!(
            fs::read(directory.join("계획서.pdf.tmp")).unwrap(),
            b"someone else's temp"
        );

        // Sixteen saves of one name at once: sixteen different files, no loss.
        let handles: Vec<_> = (0..16)
            .map(|index| {
                let directory = directory.clone();
                std::thread::spawn(move || {
                    let body = format!("copy-{index}");
                    let path =
                        write_new_file_in(&directory, "보고서.txt", body.as_bytes()).unwrap();
                    (path, body)
                })
            })
            .collect();
        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        let names: std::collections::HashSet<_> = results.iter().map(|(p, _)| p.clone()).collect();
        assert_eq!(names.len(), 16, "every save got its own file");
        for (path, body) in &results {
            assert_eq!(
                fs::read_to_string(path).unwrap(),
                *body,
                "no save was overwritten"
            );
        }

        fs::remove_dir_all(&directory).unwrap();
    }

    /// Unsigned JWT-shaped token carrying only a subject, for local checks.
    fn test_token(subject: &str) -> String {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none","typ":"JWT"}"#);
        let claims = URL_SAFE_NO_PAD.encode(format!(r#"{{"sub":"{subject}"}}"#));
        format!("{header}.{claims}.signature")
    }

    #[test]
    fn rejects_non_http_target() {
        let error = validate_target_url("file:///tmp/example").unwrap_err();
        assert!(error.contains("http"));
    }

    #[test]
    fn rejects_embedded_credentials() {
        let error = validate_target_url("https://user:secret@example.invalid/").unwrap_err();
        assert!(error.contains("계정 정보"));
    }

    #[test]
    fn identity_base_url_allows_https_and_a_loopback_test_provider() {
        assert!(validate_identity_base_url("https://project.supabase.co/auth/v1/").is_ok());
        assert!(validate_identity_base_url("http://127.0.0.1:54321/auth/v1/").is_ok());
        // A base without the trailing slash still resolves `authorize`/`token`
        // under the same prefix.
        assert_eq!(
            validate_identity_base_url("https://project.supabase.co/auth/v1")
                .unwrap()
                .as_str(),
            "https://project.supabase.co/auth/v1/"
        );
    }

    #[test]
    fn identity_base_url_refuses_plain_http_elsewhere_and_embedded_credentials() {
        for value in [
            "http://project.supabase.co/auth/v1/",
            "http://[::1]:54321/auth/v1/",
            "ftp://project.supabase.co/auth/v1/",
            "https://user:secret@project.supabase.co/auth/v1/",
            "not a url",
        ] {
            assert!(
                validate_identity_base_url(value).is_err(),
                "accepted identity base URL: {value}"
            );
        }
    }

    #[test]
    fn anon_key_must_look_like_one_header_value() {
        assert!(validate_anon_key("sb_publishable_example_key_for_a_project").is_ok());
        assert!(validate_anon_key("").is_err());
        assert!(validate_anon_key("   ").is_err());
        assert!(validate_anon_key("key with space").is_err());
        assert!(validate_anon_key("key\nInjected: 1").is_err());
        assert!(validate_anon_key(&"k".repeat(MAX_ANON_KEY_LEN + 1)).is_err());
    }

    #[test]
    fn recipe_round_trip_preserves_fields() {
        let recipe = AutomationRecipe {
            id: "recipe-1".to_string(),
            name: "기안".to_string(),
            target_url: "https://example.invalid/draft".to_string(),
            kind: AutomationKind::Shortcut,
            fields: Vec::new(),
            table: None,
            watch: None,
        };
        let decoded = decode_recipe(&encode_recipe(&recipe).unwrap()).unwrap();
        assert_eq!(decoded, recipe);
    }

    fn fill_field(label: &str, kind: LocatorKind, value: &str) -> AutomationField {
        AutomationField {
            label: label.to_string(),
            locator: AutomationLocator {
                kind,
                value: value.to_string(),
            },
        }
    }

    fn fill_recipe(id: &str, name: &str, fields: Vec<AutomationField>) -> AutomationRecipe {
        AutomationRecipe {
            id: id.to_string(),
            name: name.to_string(),
            target_url: "https://example.invalid/list".to_string(),
            kind: AutomationKind::Fill,
            fields,
            table: None,
            watch: None,
        }
    }

    fn table_recipe(id: &str, name: &str) -> AutomationRecipe {
        AutomationRecipe {
            id: id.to_string(),
            name: name.to_string(),
            target_url: "https://example.invalid/attendance".to_string(),
            kind: AutomationKind::TableFill,
            fields: Vec::new(),
            table: Some(AutomationTable {
                locator: AutomationLocator {
                    kind: LocatorKind::Css,
                    value: "#attendance".to_string(),
                },
                identity_headers: vec!["학년".to_string(), "반".to_string(), "이름".to_string()],
                date_labels: vec!["3-2".to_string(), "3-3".to_string()],
            }),
            watch: None,
        }
    }

    #[test]
    fn table_recipe_round_trip_keeps_headers() {
        let recipe = table_recipe("recipe-table", "출결 입력");

        let decoded = decode_recipe(&encode_recipe(&recipe).unwrap()).unwrap();

        assert_eq!(decoded, recipe);
        assert_eq!(
            decoded.table.as_ref().unwrap().date_labels,
            vec!["3-2".to_string(), "3-3".to_string()]
        );
    }

    #[test]
    fn decodes_the_earlier_field_array_payload() {
        let line = "recipe-1\tfill\t기안 작성\thttps://example.invalid/list\t\
                    [{\"label\":\"제목\",\"locator\":{\"kind\":\"id\",\"value\":\"title\"}}]";

        let decoded = decode_recipe(line).unwrap();

        assert_eq!(decoded.kind, AutomationKind::Fill);
        assert_eq!(decoded.fields.len(), 1);
        assert!(decoded.table.is_none());
    }

    #[test]
    fn rejects_table_recipes_without_table_or_with_fields() {
        let mut missing = table_recipe("recipe-table", "출결 입력");
        missing.table = None;
        assert!(validate_recipe(&missing).is_err());

        let mut with_fields = table_recipe("recipe-table", "출결 입력");
        with_fields.fields = vec![fill_field("제목", LocatorKind::Id, "title")];
        assert!(validate_recipe(&with_fields).is_err());

        let mut shortcut = shortcut_recipe("recipe-1", "기안", "https://example.invalid/draft");
        shortcut.table = table_recipe("recipe-table", "출결 입력").table;
        assert!(validate_recipe(&shortcut).is_err());

        let mut fill = fill_recipe(
            "recipe-fill",
            "기안 작성",
            vec![fill_field("제목", LocatorKind::Id, "title")],
        );
        fill.table = table_recipe("recipe-table", "출결 입력").table;
        assert!(validate_recipe(&fill).is_err());
    }

    #[test]
    fn rejects_table_recipes_beyond_header_limits() {
        let mut recipe = table_recipe("recipe-table", "출결 입력");
        let table = recipe.table.as_mut().unwrap();
        table.identity_headers = vec!["이름".to_string(); MAX_TABLE_HEADERS + 1];
        assert!(validate_recipe(&recipe).is_err());

        let mut recipe = table_recipe("recipe-table", "출결 입력");
        let table = recipe.table.as_mut().unwrap();
        table.date_labels = vec!["3-2".to_string(); MAX_TABLE_DATES + 1];
        assert!(validate_recipe(&recipe).is_err());

        let mut recipe = table_recipe("recipe-table", "출결 입력");
        recipe.table.as_mut().unwrap().identity_headers = vec!["  ".to_string()];
        assert!(validate_recipe(&recipe).is_err());
    }

    fn watch_recipe(id: &str, name: &str) -> AutomationRecipe {
        AutomationRecipe {
            id: id.to_string(),
            name: name.to_string(),
            target_url: "https://example.invalid/experience".to_string(),
            kind: AutomationKind::Watch,
            fields: Vec::new(),
            table: None,
            watch: Some(AutomationWatch {
                list: AutomationLocator {
                    kind: LocatorKind::Css,
                    value: "#applications > tbody > tr".to_string(),
                },
                identity: AutomationLocator {
                    kind: LocatorKind::Css,
                    value: "td:nth-child(1)".to_string(),
                },
            }),
        }
    }

    #[test]
    fn watch_recipe_round_trip_keeps_locators() {
        let recipe = watch_recipe("recipe-watch", "체험학습 신청 감시");

        let decoded = decode_recipe(&encode_recipe(&recipe).unwrap()).unwrap();

        assert_eq!(decoded, recipe);
        assert_eq!(decoded.kind, AutomationKind::Watch);
        assert_eq!(
            decoded.watch.as_ref().unwrap().list.value,
            "#applications > tbody > tr"
        );
    }

    #[test]
    fn rejects_watch_recipes_with_other_payloads() {
        let mut missing = watch_recipe("recipe-watch", "체험학습 신청 감시");
        missing.watch = None;
        assert!(validate_recipe(&missing).is_err());

        let mut with_fields = watch_recipe("recipe-watch", "체험학습 신청 감시");
        with_fields.fields = vec![fill_field("제목", LocatorKind::Id, "title")];
        assert!(validate_recipe(&with_fields).is_err());

        let mut with_table = watch_recipe("recipe-watch", "체험학습 신청 감시");
        with_table.table = table_recipe("recipe-table", "출결 입력").table;
        assert!(validate_recipe(&with_table).is_err());

        // 목록 위치는 여러 행을 가리켜야 하므로 css만 받습니다.
        let mut non_css_list = watch_recipe("recipe-watch", "체험학습 신청 감시");
        non_css_list.watch.as_mut().unwrap().list = AutomationLocator {
            kind: LocatorKind::Id,
            value: "applications".to_string(),
        };
        assert!(validate_recipe(&non_css_list).is_err());

        // 다른 종류에는 감시 정보를 둘 수 없습니다.
        let mut shortcut = shortcut_recipe("recipe-1", "기안", "https://example.invalid/draft");
        shortcut.watch = watch_recipe("recipe-watch", "감시").watch;
        assert!(validate_recipe(&shortcut).is_err());

        let mut table = table_recipe("recipe-table", "출결 입력");
        table.watch = watch_recipe("recipe-watch", "감시").watch;
        assert!(validate_recipe(&table).is_err());
    }

    #[test]
    fn watch_scan_reports_only_new_identifiers() {
        let directory = test_directory("watch-scan");
        let path = directory.join(AUTOMATION_WATCH_STATE_FILE);

        let first = record_watch_scan(
            &path,
            "recipe-watch",
            vec!["2026-01".to_string(), "2026-02".to_string()],
        )
        .unwrap();
        assert_eq!(first.new_identifiers, vec!["2026-01", "2026-02"]);
        assert_eq!(first.total, 2);
        assert!(first.checked_at_ms > 0);

        // 같은 목록을 다시 보면 새 항목이 없어야 합니다.
        let second = record_watch_scan(
            &path,
            "recipe-watch",
            vec!["2026-02".to_string(), "2026-01".to_string()],
        )
        .unwrap();
        assert!(second.new_identifiers.is_empty());

        // 새 식별자만 알립니다. 중복 값은 한 번으로 정리합니다.
        let third = record_watch_scan(
            &path,
            "recipe-watch",
            vec![
                "2026-01".to_string(),
                "2026-02".to_string(),
                "2026-03".to_string(),
                "2026-03".to_string(),
            ],
        )
        .unwrap();
        assert_eq!(third.new_identifiers, vec!["2026-03"]);
        assert_eq!(third.total, 3);

        // 확인 처리하면 새 항목 표시가 비워집니다.
        let entries = acknowledge_watch(&path, "recipe-watch").unwrap();
        assert!(entries[0].new_identifiers.is_empty());
        assert_eq!(entries[0].identifiers.len(), 3);

        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn paused_watch_scan_keeps_previous_state() {
        let directory = test_directory("watch-paused");
        let path = directory.join(AUTOMATION_WATCH_STATE_FILE);

        record_watch_scan(&path, "recipe-watch", vec!["2026-01".to_string()]).unwrap();
        let entries = set_watch_paused(&path, "recipe-watch", true).unwrap();
        assert!(entries[0].paused);
        let checked_before = entries[0].checked_at_ms;

        let paused = record_watch_scan(&path, "recipe-watch", vec!["2026-09".to_string()]).unwrap();
        assert!(paused.paused);
        assert!(paused.new_identifiers.is_empty());

        let entries = read_watch_state(&path).unwrap();
        assert_eq!(entries[0].identifiers, vec!["2026-01"]);
        assert_eq!(entries[0].checked_at_ms, checked_before);

        // 다시 켜면 그 사이에 생긴 항목을 새 항목으로 알립니다.
        set_watch_paused(&path, "recipe-watch", false).unwrap();
        let resumed =
            record_watch_scan(&path, "recipe-watch", vec!["2026-09".to_string()]).unwrap();
        assert_eq!(resumed.new_identifiers, vec!["2026-09"]);

        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn corrupt_watch_state_is_quarantined_and_reset() {
        let directory = test_directory("watch-corrupt");
        let path = directory.join(AUTOMATION_WATCH_STATE_FILE);
        fs::write(&path, "{ not json").unwrap();

        let entries = read_watch_state(&path).unwrap();

        assert!(entries.is_empty());
        assert!(!path.exists());
        assert!(
            directory
                .join(AUTOMATION_WATCH_STATE_QUARANTINE_FILE)
                .exists()
        );

        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn fill_recipe_round_trip_keeps_locators() {
        let recipe = fill_recipe(
            "recipe-fill",
            "기안 작성",
            vec![
                fill_field("제목", LocatorKind::Id, "title"),
                fill_field("담당", LocatorKind::Name, "owner"),
                fill_field("기안일", LocatorKind::Label, "기안일"),
                fill_field(
                    "비고",
                    LocatorKind::Css,
                    "#main > form > div:nth-of-type(2) > textarea",
                ),
            ],
        );

        let decoded = decode_recipe(&encode_recipe(&recipe).unwrap()).unwrap();

        assert_eq!(decoded, recipe);
        assert_eq!(decoded.fields.len(), 4);
    }

    #[test]
    fn rejects_fields_on_a_shortcut_recipe() {
        let mut recipe = shortcut_recipe("recipe-1", "기안", "https://example.invalid/draft");
        recipe.fields = vec![fill_field("제목", LocatorKind::Id, "title")];

        assert!(validate_recipe(&recipe).is_err());
    }

    #[test]
    fn rejects_a_fill_recipe_without_fields() {
        let recipe = fill_recipe("recipe-fill", "기안 작성", Vec::new());

        assert!(validate_recipe(&recipe).is_err());
    }

    #[test]
    fn rejects_unsafe_locator_values() {
        for field in [
            fill_field("제목", LocatorKind::Css, "div; body"),
            fill_field("제목", LocatorKind::Css, "div, span"),
            fill_field("제목", LocatorKind::Css, ":is(div)"),
            fill_field("제목", LocatorKind::Css, "div:has(> input)"),
            fill_field("제목", LocatorKind::Id, "title with space"),
            fill_field("제목", LocatorKind::Name, "a\"b"),
            fill_field("제목", LocatorKind::Label, ""),
        ] {
            let recipe = fill_recipe("recipe-fill", "기안 작성", vec![field.clone()]);
            assert!(
                validate_recipe(&recipe).is_err(),
                "accepted locator: {field:?}"
            );
        }
    }

    #[test]
    fn rejects_a_fill_recipe_beyond_the_field_limit() {
        let fields = (0..=MAX_RECIPE_FIELDS)
            .map(|index| fill_field(&format!("필드{index}"), LocatorKind::Id, "title"))
            .collect::<Vec<_>>();
        let recipe = fill_recipe("recipe-fill", "기안 작성", fields);

        assert!(validate_recipe(&recipe).is_err());
    }

    #[test]
    fn normalizes_field_whitespace_before_saving() {
        let recipe = normalize_recipe(fill_recipe(
            " recipe-fill ",
            " 기안 작성 ",
            vec![fill_field(" 제목 ", LocatorKind::Id, " title ")],
        ))
        .unwrap();

        assert_eq!(recipe.id, "recipe-fill");
        assert_eq!(recipe.name, "기안 작성");
        assert_eq!(recipe.fields[0].label, "제목");
        assert_eq!(recipe.fields[0].locator.value, "title");
    }

    #[test]
    fn keeps_valid_lines_and_reports_damaged_lines() {
        let content = concat!(
            "recipe-1\tshortcut\t기안\thttps://example.invalid/draft\n",
            "damaged-line-without-fields\n",
            "\n",
            "recipe-2\tshortcut\t품의\thttps://example.invalid/approval\n",
        );

        let (recipes, rejected) = decode_recipes(content);

        assert_eq!(recipes.len(), 2);
        assert_eq!(recipes[0].id, "recipe-1");
        assert_eq!(recipes[1].id, "recipe-2");
        assert_eq!(rejected, vec!["damaged-line-without-fields".to_string()]);
    }

    #[test]
    fn damaged_line_does_not_hide_other_recipes() {
        let content = concat!(
            "recipe-1\tshortcut\t기안\thttps://user:secret@example.invalid/draft\n",
            "recipe-2\tshortcut\t품의\thttps://example.invalid/approval\n",
        );

        let (recipes, rejected) = decode_recipes(content);

        assert_eq!(recipes.len(), 1);
        assert_eq!(recipes[0].id, "recipe-2");
        assert_eq!(rejected.len(), 1);
    }

    #[test]
    fn quarantine_appends_damaged_lines() {
        let directory =
            std::env::temp_dir().join(format!("school-collect-quarantine-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();

        quarantine_invalid_lines(&directory, &["broken-1".to_string()]).unwrap();
        quarantine_invalid_lines(&directory, &["broken-2".to_string()]).unwrap();

        let content =
            fs::read_to_string(directory.join(AUTOMATION_RECIPES_QUARANTINE_FILE)).unwrap();
        assert_eq!(content, "broken-1\nbroken-2\n");

        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn atomic_write_replaces_file_without_leaving_temp_file() {
        let directory =
            std::env::temp_dir().join(format!("school-collect-atomic-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join(AUTOMATION_RECIPES_FILE);
        fs::write(&path, "stale-content\n").unwrap();

        write_file_atomically(&path, "fresh-content\n").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "fresh-content\n");
        assert!(
            !directory
                .join(format!("{AUTOMATION_RECIPES_FILE}.tmp"))
                .exists()
        );

        fs::remove_dir_all(&directory).unwrap();
    }

    fn test_directory(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("school-collect-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    const INVALID_UTF8_BYTES: [u8; 4] = [0xFF, 0xFE, 0x62, 0x72];

    fn shortcut_recipe(id: &str, name: &str, target_url: &str) -> AutomationRecipe {
        AutomationRecipe {
            id: id.to_string(),
            name: name.to_string(),
            target_url: target_url.to_string(),
            kind: AutomationKind::Shortcut,
            fields: Vec::new(),
            table: None,
            watch: None,
        }
    }

    fn damaged_recipes_file() -> &'static str {
        concat!(
            "recipe-1\tshortcut\t기안\thttps://example.invalid/draft\n",
            "damaged-line\n",
        )
    }

    #[test]
    fn quarantine_read_failure_keeps_existing_quarantine_bytes() {
        let directory = test_directory("quarantine-read-failure");
        let quarantine_path = directory.join(AUTOMATION_RECIPES_QUARANTINE_FILE);
        fs::write(&quarantine_path, INVALID_UTF8_BYTES).unwrap();

        let result = quarantine_invalid_lines(&directory, &["damaged-line".to_string()]);

        assert!(result.is_err());
        assert_eq!(fs::read(&quarantine_path).unwrap(), INVALID_UTF8_BYTES);
    }

    #[test]
    fn quarantine_failure_keeps_original_bytes_for_save_and_delete() {
        let directory = test_directory("quarantine-failure");
        let recipes_path = directory.join(AUTOMATION_RECIPES_FILE);
        let quarantine_path = directory.join(AUTOMATION_RECIPES_QUARANTINE_FILE);
        let original = damaged_recipes_file();
        fs::write(&recipes_path, original).unwrap();
        // 격리 경로를 디렉터리로 만들어 격리를 실패시킵니다.
        fs::create_dir(&quarantine_path).unwrap();

        let saved = upsert_automation_recipe(
            &recipes_path,
            shortcut_recipe("recipe-2", "품의", "https://example.invalid/approval"),
        );
        let removed = remove_automation_recipe(&recipes_path, "recipe-1");

        assert!(saved.is_err());
        assert!(removed.is_err());
        assert_eq!(fs::read(&recipes_path).unwrap(), original.as_bytes());
        assert!(quarantine_path.is_dir());
    }

    #[test]
    fn listing_keeps_valid_recipes_when_recovery_is_blocked() {
        let directory = test_directory("list-blocked-recovery");
        let recipes_path = directory.join(AUTOMATION_RECIPES_FILE);
        let original = damaged_recipes_file();
        fs::write(&recipes_path, original).unwrap();
        fs::create_dir(directory.join(AUTOMATION_RECIPES_QUARANTINE_FILE)).unwrap();

        let file = RecipeFile::read(&recipes_path).unwrap();

        assert_eq!(file.recipes.len(), 1);
        assert_eq!(file.damaged_lines.len(), 1);
        assert!(file.recover(&recipes_path).is_err());
        // 조회 경로는 원본을 덮어쓰지 않습니다.
        assert_eq!(fs::read(&recipes_path).unwrap(), original.as_bytes());
    }

    #[test]
    fn recovery_then_save_succeeds() {
        let directory = test_directory("recovery-then-save");
        let recipes_path = directory.join(AUTOMATION_RECIPES_FILE);
        let quarantine_path = directory.join(AUTOMATION_RECIPES_QUARANTINE_FILE);
        fs::write(&recipes_path, damaged_recipes_file()).unwrap();

        let recipes = upsert_automation_recipe(
            &recipes_path,
            shortcut_recipe("recipe-2", "품의", "https://example.invalid/approval"),
        )
        .unwrap();

        assert_eq!(recipes.len(), 2);
        assert_eq!(
            fs::read_to_string(&quarantine_path).unwrap(),
            "damaged-line\n"
        );

        let stored = fs::read_to_string(&recipes_path).unwrap();
        assert!(stored.contains("recipe-1"));
        assert!(stored.contains("recipe-2"));
        assert!(!stored.contains("damaged-line"));

        let remaining = remove_automation_recipe(&recipes_path, "recipe-1").unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, "recipe-2");
    }

    #[test]
    fn normalizes_scheme_host_and_default_port() {
        let normalized = normalize_target_url("HTTPS://Example.INVALID:443/Path").unwrap();
        assert_eq!(normalized, "https://example.invalid/Path");
    }

    #[test]
    fn rejects_target_without_host() {
        assert!(validate_target_url("https://:8080/portal").is_err());
    }

    #[test]
    fn rejects_unsupported_scheme() {
        assert!(validate_target_url("ftp://example.invalid/portal").is_err());
    }

    #[test]
    fn trims_fields_when_decoding() {
        let decoded =
            decode_recipe(" recipe-1 \tshortcut\t 기안 \t https://example.invalid/draft ").unwrap();

        assert_eq!(decoded.id, "recipe-1");
        assert_eq!(decoded.name, "기안");
        assert_eq!(decoded.target_url, "https://example.invalid/draft");
    }

    #[test]
    fn rejects_new_recipe_beyond_limit_but_updates_existing() {
        let directory = test_directory("recipe-limit");
        let recipes_path = directory.join(AUTOMATION_RECIPES_FILE);
        let recipes = (0..MAX_RECIPES)
            .map(|index| {
                shortcut_recipe(
                    &format!("recipe-{index}"),
                    &format!("버튼{index}"),
                    "https://example.invalid/draft",
                )
            })
            .collect::<Vec<_>>();
        write_automation_recipes(&recipes_path, &recipes).unwrap();

        let error = upsert_automation_recipe(
            &recipes_path,
            shortcut_recipe("recipe-extra", "추가", "https://example.invalid/draft"),
        )
        .unwrap_err();
        assert!(error.contains("최대"));

        let updated = upsert_automation_recipe(
            &recipes_path,
            shortcut_recipe("recipe-0", "이름변경", "https://example.invalid/draft"),
        )
        .unwrap();
        assert_eq!(updated.len(), MAX_RECIPES);
    }

    #[test]
    fn quarantine_size_limit_preserves_existing_bytes() {
        let directory = test_directory("quarantine-limit");
        let quarantine_path = directory.join(AUTOMATION_RECIPES_QUARANTINE_FILE);
        fs::write(&quarantine_path, "x".repeat(MAX_QUARANTINE_BYTES)).unwrap();

        let result = quarantine_invalid_lines(&directory, &["damaged-line".to_string()]);

        assert!(result.is_err());
        assert_eq!(
            fs::read(&quarantine_path).unwrap().len(),
            MAX_QUARANTINE_BYTES
        );
    }

    #[cfg(unix)]
    #[test]
    fn recipe_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let directory = test_directory("recipe-permissions");
        let recipes_path = directory.join(AUTOMATION_RECIPES_FILE);

        write_file_atomically(
            &recipes_path,
            "recipe-1\tshortcut\t기안\thttps://example.invalid/draft\n",
        )
        .unwrap();

        let mode = fs::metadata(&recipes_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn resolves_target_from_stored_recipe_id() {
        let directory = test_directory("resolve-target");
        let recipes_path = directory.join(AUTOMATION_RECIPES_FILE);
        upsert_automation_recipe(
            &recipes_path,
            shortcut_recipe("recipe-1", "기안", "https://example.invalid/draft"),
        )
        .unwrap();

        let target = resolve_recipe_target(&recipes_path, "recipe-1").unwrap();

        assert_eq!(target, "https://example.invalid/draft");
    }

    #[test]
    fn rejects_unknown_recipe_id() {
        let directory = test_directory("resolve-unknown");
        let recipes_path = directory.join(AUTOMATION_RECIPES_FILE);
        upsert_automation_recipe(
            &recipes_path,
            shortcut_recipe("recipe-1", "기안", "https://example.invalid/draft"),
        )
        .unwrap();

        let error = resolve_recipe_target(&recipes_path, "recipe-2").unwrap_err();

        assert!(error.contains("찾지 못했습니다"));
    }

    #[test]
    fn damaged_file_still_resolves_remaining_recipe() {
        let directory = test_directory("resolve-damaged");
        let recipes_path = directory.join(AUTOMATION_RECIPES_FILE);
        fs::write(&recipes_path, damaged_recipes_file()).unwrap();
        // 격리 경로를 디렉터리로 만들어 복구를 실패시킵니다.
        fs::create_dir(directory.join(AUTOMATION_RECIPES_QUARANTINE_FILE)).unwrap();

        let target = resolve_recipe_target(&recipes_path, "recipe-1").unwrap();

        assert_eq!(target, "https://example.invalid/draft");
    }
}
