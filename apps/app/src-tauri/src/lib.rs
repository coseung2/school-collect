use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::Manager;
use url::Url;

const AUTOMATION_RECIPES_FILE: &str = "automation-recipes.json";
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

fn validate_target_url(value: &str) -> Result<Url, String> {
    if value.len() > MAX_TARGET_URL_LEN {
        return Err("대상 URL이 너무 깁니다.".to_string());
    }

    let parsed = Url::parse(value).map_err(|_| "올바른 URL을 입력하세요.".to_string())?;
    if parsed.scheme() != "https" && parsed.scheme() != "http" {
        return Err("http 또는 https URL만 등록할 수 있습니다.".to_string());
    }
    if parsed.host_str().is_none() {
        return Err("대상 URL에 호스트가 없습니다.".to_string());
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("계정 정보가 포함된 URL은 등록할 수 없습니다.".to_string());
    }

    Ok(parsed)
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
    if recipe.kind != AutomationKind::Shortcut {
        return Err("지원하지 않는 자동화 유형입니다.".to_string());
    }

    validate_target_url(recipe.target_url.trim())?;
    Ok(())
}

fn automation_recipes_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let directory = app_handle
        .path()
        .app_config_dir()
        .map_err(|error| format!("자동화 설정 경로를 확인하지 못했습니다: {error}"))?;

    fs::create_dir_all(&directory)
        .map_err(|error| format!("자동화 설정 디렉터리를 만들지 못했습니다: {error}"))?;

    Ok(directory.join(AUTOMATION_RECIPES_FILE))
}

fn read_automation_recipes(app_handle: &tauri::AppHandle) -> Result<Vec<AutomationRecipe>, String> {
    let path = automation_recipes_path(app_handle)?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(path)
        .map_err(|error| format!("자동화 설정을 읽지 못했습니다: {error}"))?;
    let recipes: Vec<AutomationRecipe> = serde_json::from_str(&content)
        .map_err(|error| format!("자동화 설정 형식이 올바르지 않습니다: {error}"))?;

    for recipe in &recipes {
        validate_recipe(recipe)?;
    }

    Ok(recipes)
}

fn write_automation_recipes(
    app_handle: &tauri::AppHandle,
    recipes: &[AutomationRecipe],
) -> Result<(), String> {
    let path = automation_recipes_path(app_handle)?;
    let content = serde_json::to_string_pretty(recipes)
        .map_err(|error| format!("자동화 설정을 직렬화하지 못했습니다: {error}"))?;

    fs::write(path, content)
        .map_err(|error| format!("자동화 설정을 저장하지 못했습니다: {error}"))
}

#[tauri::command]
fn list_automation_recipes(
    app_handle: tauri::AppHandle,
) -> Result<Vec<AutomationRecipe>, String> {
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
    let parsed = validate_target_url(target_url.trim())?;
    launch_external_url(parsed.as_str())
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
        let url = validate_target_url("https://example.invalid/portal#draft").unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("example.invalid"));
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
}
