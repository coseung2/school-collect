use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use tauri::Manager;
use url::Url;

mod bridge;

pub(crate) const AUTOMATION_RECIPES_FILE: &str = "automation-recipes.tsv";
const AUTOMATION_RECIPES_QUARANTINE_FILE: &str = "automation-recipes.invalid.tsv";
const MAX_RECIPE_NAME_LEN: usize = 80;
const MAX_RECIPE_ID_LEN: usize = 128;
const MAX_TARGET_URL_LEN: usize = 2048;
/// 한 사용자 계정이 보관할 수 있는 업무 버튼 수입니다.
const MAX_RECIPES: usize = 100;
/// 손상된 행을 보관하는 격리 파일의 상한입니다. 넘으면 복구를 멈추고 원본을 보존합니다.
const MAX_QUARANTINE_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AutomationKind {
    Shortcut,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AutomationRecipe {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) target_url: String,
    pub(crate) kind: AutomationKind,
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
    Ok(())
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

fn encode_recipe(recipe: &AutomationRecipe) -> String {
    format!(
        "{}\tshortcut\t{}\t{}",
        recipe.id, recipe.name, recipe.target_url
    )
}

fn decode_recipe(line: &str) -> Result<AutomationRecipe, String> {
    let mut fields = line.splitn(4, '\t');
    // 저장·검증과 같은 기준을 쓰도록 읽을 때도 값을 trim합니다.
    let id = fields.next().unwrap_or("").trim();
    let kind = fields.next().unwrap_or("").trim();
    let name = fields.next().unwrap_or("").trim();
    let target_url = fields.next().unwrap_or("").trim();

    if kind != "shortcut" || id.is_empty() || name.is_empty() || target_url.is_empty() {
        return Err("자동화 설정 형식이 올바르지 않습니다.".to_string());
    }

    let recipe = AutomationRecipe {
        id: id.to_string(),
        kind: AutomationKind::Shortcut,
        name: name.to_string(),
        target_url: target_url.to_string(),
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
    let mut content = recipes
        .iter()
        .map(encode_recipe)
        .collect::<Vec<_>>()
        .join("\n");
    if !content.is_empty() {
        content.push('\n');
    }

    write_file_atomically(path, &content)
}

fn write_temp_file(path: &Path, content: &str) -> std::io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()
}

pub(crate) fn write_file_atomically(path: &Path, content: &str) -> Result<(), String> {
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
    mut recipe: AutomationRecipe,
) -> Result<Vec<AutomationRecipe>, String> {
    recipe.id = recipe.id.trim().to_string();
    recipe.name = recipe.name.trim().to_string();
    recipe.target_url = normalize_target_url(recipe.target_url.trim())?;
    validate_recipe(&recipe)?;

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
    remove_automation_recipe(&path, recipe_id)
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
            open_automation_recipe
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
    fn recipe_round_trip_preserves_fields() {
        let recipe = AutomationRecipe {
            id: "recipe-1".to_string(),
            name: "기안".to_string(),
            target_url: "https://example.invalid/draft".to_string(),
            kind: AutomationKind::Shortcut,
        };
        let decoded = decode_recipe(&encode_recipe(&recipe)).unwrap();
        assert_eq!(decoded, recipe);
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
