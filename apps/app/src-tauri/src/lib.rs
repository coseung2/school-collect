use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use tauri::Manager;

const AUTOMATION_RECIPES_FILE: &str = "automation-recipes.tsv";
const AUTOMATION_RECIPES_QUARANTINE_FILE: &str = "automation-recipes.invalid.tsv";
const MAX_RECIPE_NAME_LEN: usize = 80;
const MAX_RECIPE_ID_LEN: usize = 128;
const MAX_TARGET_URL_LEN: usize = 2048;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum AutomationKind {
    Shortcut,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct AutomationRecipe {
    id: String,
    name: String,
    target_url: String,
    kind: AutomationKind,
}

fn validate_plain_field(value: &str) -> Result<(), String> {
    if value.contains(['\t', '\r', '\n']) {
        return Err("탭이나 줄바꿈 문자는 사용할 수 없습니다.".to_string());
    }
    Ok(())
}

fn validate_target_url(value: &str) -> Result<(), String> {
    if value.len() > MAX_TARGET_URL_LEN {
        return Err("대상 URL이 너무 깁니다.".to_string());
    }
    validate_plain_field(value)?;

    let authority = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
        .ok_or_else(|| "http 또는 https URL만 등록할 수 있습니다.".to_string())?
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("");

    if authority.is_empty() {
        return Err("대상 URL에 호스트가 없습니다.".to_string());
    }
    if authority.contains('@') {
        return Err("계정 정보가 포함된 URL은 등록할 수 없습니다.".to_string());
    }

    Ok(())
}

fn validate_recipe(recipe: &AutomationRecipe) -> Result<(), String> {
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

fn automation_config_dir(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let directory = app_handle
        .path()
        .app_config_dir()
        .map_err(|error| format!("자동화 설정 경로를 확인하지 못했습니다: {error}"))?;

    fs::create_dir_all(&directory)
        .map_err(|error| format!("자동화 설정 디렉터리를 만들지 못했습니다: {error}"))?;

    Ok(directory)
}

fn automation_recipes_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
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
    let id = fields.next().unwrap_or("");
    let kind = fields.next().unwrap_or("");
    let name = fields.next().unwrap_or("");
    let target_url = fields.next().unwrap_or("");

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
    let mut content = fs::read_to_string(&path).unwrap_or_default();

    for line in lines {
        content.push_str(line);
        content.push('\n');
    }

    write_file_atomically(&path, &content)
}

fn read_automation_recipes(app_handle: &tauri::AppHandle) -> Result<Vec<AutomationRecipe>, String> {
    let path = automation_recipes_path(app_handle)?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&path)
        .map_err(|error| format!("자동화 설정을 읽지 못했습니다: {error}"))?;
    let (recipes, rejected) = decode_recipes(&content);

    if !rejected.is_empty() {
        let directory = automation_config_dir(app_handle)?;
        // 손상된 행을 먼저 격리한 뒤에만 원본을 정리합니다.
        // 격리에 실패하면 원본을 그대로 남겨 값을 잃지 않습니다.
        if quarantine_invalid_lines(&directory, &rejected).is_ok() {
            let _ = write_automation_recipes(app_handle, &recipes);
        }
    }

    Ok(recipes)
}

fn write_automation_recipes(
    app_handle: &tauri::AppHandle,
    recipes: &[AutomationRecipe],
) -> Result<(), String> {
    let path = automation_recipes_path(app_handle)?;
    let mut content = recipes
        .iter()
        .map(encode_recipe)
        .collect::<Vec<_>>()
        .join("\n");
    if !content.is_empty() {
        content.push('\n');
    }

    write_file_atomically(&path, &content)
}

fn write_temp_file(path: &Path, content: &str) -> std::io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()
}

fn write_file_atomically(path: &Path, content: &str) -> Result<(), String> {
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

    if let Err(error) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(format!("자동화 설정을 저장하지 못했습니다: {error}"));
    }

    Ok(())
}

#[tauri::command]
fn list_automation_recipes(app_handle: tauri::AppHandle) -> Result<Vec<AutomationRecipe>, String> {
    read_automation_recipes(&app_handle)
}

#[tauri::command]
fn save_automation_recipe(
    app_handle: tauri::AppHandle,
    mut recipe: AutomationRecipe,
) -> Result<Vec<AutomationRecipe>, String> {
    recipe.id = recipe.id.trim().to_string();
    recipe.name = recipe.name.trim().to_string();
    recipe.target_url = recipe.target_url.trim().to_string();
    validate_recipe(&recipe)?;

    let mut recipes = read_automation_recipes(&app_handle)?;
    if let Some(existing) = recipes.iter_mut().find(|item| item.id == recipe.id) {
        *existing = recipe;
    } else {
        recipes.push(recipe);
    }
    recipes.sort_by(|left, right| left.name.cmp(&right.name));

    write_automation_recipes(&app_handle, &recipes)?;
    Ok(recipes)
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

    let mut recipes = read_automation_recipes(&app_handle)?;
    recipes.retain(|recipe| recipe.id != recipe_id);
    write_automation_recipes(&app_handle, &recipes)?;
    Ok(recipes)
}

#[cfg(target_os = "windows")]
fn launch_external_url(url: &str) -> Result<(), String> {
    let status = std::process::Command::new("rundll32")
        .arg("url.dll,FileProtocolHandler")
        .arg(url)
        .status()
        .map_err(|error| format!("브라우저를 열지 못했습니다: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err("브라우저 열기 명령이 실패했습니다.".to_string())
    }
}

#[cfg(target_os = "macos")]
fn launch_external_url(url: &str) -> Result<(), String> {
    let status = std::process::Command::new("open")
        .arg(url)
        .status()
        .map_err(|error| format!("브라우저를 열지 못했습니다: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err("브라우저 열기 명령이 실패했습니다.".to_string())
    }
}

#[cfg(target_os = "linux")]
fn launch_external_url(url: &str) -> Result<(), String> {
    let status = std::process::Command::new("xdg-open")
        .arg(url)
        .status()
        .map_err(|error| format!("브라우저를 열지 못했습니다: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err("브라우저 열기 명령이 실패했습니다.".to_string())
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn launch_external_url(_url: &str) -> Result<(), String> {
    Err("현재 플랫폼에서는 외부 브라우저 열기를 지원하지 않습니다.".to_string())
}

#[tauri::command]
fn open_automation_target(target_url: String) -> Result<(), String> {
    let target_url = target_url.trim();
    validate_target_url(target_url)?;
    launch_external_url(target_url)
}

#[tauri::command]
fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            app_version,
            list_automation_recipes,
            save_automation_recipe,
            delete_automation_recipe,
            open_automation_target
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
}
