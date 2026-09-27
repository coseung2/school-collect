//! Local submission drafts.
//!
//! Drafts belong to one user in one school, so signing out of a shared
//! computer cannot show another person's work. The store is a plain SQLite
//! file in the app data directory: it holds text the user already typed, and
//! the server remains the source of truth once a draft is sent.

use std::{fs, path::Path};

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

/// Largest draft body kept locally, in bytes.
pub const MAX_DRAFT_BYTES: usize = 64 * 1024;

pub const DRAFTS_FILE: &str = "submission-drafts.sqlite";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalDraft {
    pub tenant_id: String,
    pub user_id: String,
    pub collect_id: String,
    /// Draft body as the API expects it.
    pub payload: serde_json::Value,
    /// Collect version the draft was written against.
    pub base_version: i64,
    /// When the draft was last written locally. The native side stamps it, so
    /// a save request may leave it out.
    #[serde(default)]
    pub updated_at_ms: u64,
}

fn open(path: &Path) -> Result<Connection, String> {
    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory)
            .map_err(|error| format!("초안 저장 경로를 만들지 못했습니다: {error}"))?;
    }
    let connection = Connection::open(path)
        .map_err(|error| format!("로컬 초안 저장소를 열지 못했습니다: {error}"))?;
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS submission_drafts (
                 tenant_id TEXT NOT NULL,
                 user_id TEXT NOT NULL,
                 collect_id TEXT NOT NULL,
                 payload TEXT NOT NULL,
                 base_version INTEGER NOT NULL,
                 updated_at_ms INTEGER NOT NULL,
                 PRIMARY KEY (tenant_id, user_id, collect_id)
             );",
        )
        .map_err(|error| format!("로컬 초안 표를 만들지 못했습니다: {error}"))?;
    Ok(connection)
}

fn validate_draft(draft: &LocalDraft) -> Result<String, String> {
    for value in [&draft.tenant_id, &draft.user_id, &draft.collect_id] {
        if value.trim().is_empty() || value.len() > 128 {
            return Err("초안 식별자가 올바르지 않습니다.".to_string());
        }
        if value.contains(['\t', '\r', '\n']) {
            return Err("초안 식별자에 사용할 수 없는 문자가 있습니다.".to_string());
        }
    }
    if draft.base_version < 0 {
        return Err("초안 기준 버전이 올바르지 않습니다.".to_string());
    }

    let payload = serde_json::to_string(&draft.payload)
        .map_err(|error| format!("초안을 저장할 수 없습니다: {error}"))?;
    if payload.len() > MAX_DRAFT_BYTES {
        return Err("초안이 너무 큽니다.".to_string());
    }
    Ok(payload)
}

/// Writes or replaces the draft for one collect.
pub fn save_draft(path: &Path, draft: &LocalDraft, now_ms: u64) -> Result<LocalDraft, String> {
    let payload = validate_draft(draft)?;
    let connection = open(path)?;
    connection
        .execute(
            "INSERT INTO submission_drafts
               (tenant_id, user_id, collect_id, payload, base_version, updated_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT (tenant_id, user_id, collect_id) DO UPDATE SET
               payload = excluded.payload,
               base_version = excluded.base_version,
               updated_at_ms = excluded.updated_at_ms",
            params![
                draft.tenant_id,
                draft.user_id,
                draft.collect_id,
                payload,
                draft.base_version,
                now_ms as i64,
            ],
        )
        .map_err(|error| format!("초안을 저장하지 못했습니다: {error}"))?;

    Ok(LocalDraft {
        updated_at_ms: now_ms,
        ..draft.clone()
    })
}

/// Drafts waiting for one user in one school.
pub fn list_drafts(path: &Path, tenant_id: &str, user_id: &str) -> Result<Vec<LocalDraft>, String> {
    let connection = open(path)?;
    let mut statement = connection
        .prepare(
            "SELECT tenant_id, user_id, collect_id, payload, base_version, updated_at_ms
             FROM submission_drafts
             WHERE tenant_id = ?1 AND user_id = ?2
             ORDER BY updated_at_ms DESC",
        )
        .map_err(|error| format!("초안을 읽지 못했습니다: {error}"))?;

    let rows = statement
        .query_map(params![tenant_id, user_id], |row| {
            let payload: String = row.get(3)?;
            Ok(LocalDraft {
                tenant_id: row.get(0)?,
                user_id: row.get(1)?,
                collect_id: row.get(2)?,
                payload: serde_json::from_str(&payload).unwrap_or(serde_json::Value::Null),
                base_version: row.get(4)?,
                updated_at_ms: row.get::<_, i64>(5)? as u64,
            })
        })
        .map_err(|error| format!("초안을 읽지 못했습니다: {error}"))?;

    let mut drafts = Vec::new();
    for row in rows {
        drafts.push(row.map_err(|error| format!("초안을 읽지 못했습니다: {error}"))?);
    }
    Ok(drafts)
}

pub fn find_draft(
    path: &Path,
    tenant_id: &str,
    user_id: &str,
    collect_id: &str,
) -> Result<Option<LocalDraft>, String> {
    Ok(list_drafts(path, tenant_id, user_id)?
        .into_iter()
        .find(|draft| draft.collect_id == collect_id))
}

/// Removes one draft after the server accepted it.
pub fn delete_draft(
    path: &Path,
    tenant_id: &str,
    user_id: &str,
    collect_id: &str,
) -> Result<(), String> {
    let connection = open(path)?;
    connection
        .execute(
            "DELETE FROM submission_drafts
             WHERE tenant_id = ?1 AND user_id = ?2 AND collect_id = ?3",
            params![tenant_id, user_id, collect_id],
        )
        .map_err(|error| format!("초안을 지우지 못했습니다: {error}"))?;
    Ok(())
}

/// Removes every draft of one user. Signing out must not leave work behind.
pub fn clear_user_drafts(path: &Path, user_id: &str) -> Result<usize, String> {
    let connection = open(path)?;
    let removed = connection
        .execute(
            "DELETE FROM submission_drafts WHERE user_id = ?1",
            params![user_id],
        )
        .map_err(|error| format!("초안을 지우지 못했습니다: {error}"))?;
    Ok(removed)
}

/// Removes one user's drafts in one school.
#[cfg(test)]
pub fn clear_tenant_drafts(path: &Path, tenant_id: &str, user_id: &str) -> Result<usize, String> {
    let connection = open(path)?;
    let removed = connection
        .execute(
            "DELETE FROM submission_drafts WHERE tenant_id = ?1 AND user_id = ?2",
            params![tenant_id, user_id],
        )
        .map_err(|error| format!("초안을 지우지 못했습니다: {error}"))?;
    Ok(removed)
}

/// Reads one value, used by the smoke checks.
#[cfg(test)]
pub fn draft_count(path: &Path) -> Result<i64, String> {
    use rusqlite::OptionalExtension;

    let connection = open(path)?;
    connection
        .query_row("SELECT count(*) FROM submission_drafts", [], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|error| format!("초안 수를 세지 못했습니다: {error}"))
        .map(|value| value.unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::{
        DRAFTS_FILE, LocalDraft, clear_tenant_drafts, clear_user_drafts, delete_draft, draft_count,
        find_draft, list_drafts, save_draft,
    };
    use serde_json::json;
    use std::path::PathBuf;

    fn test_directory(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "school-collect-drafts-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("directory");
        directory
    }

    fn draft(tenant: &str, user: &str, collect: &str, note: &str) -> LocalDraft {
        LocalDraft {
            tenant_id: tenant.to_string(),
            user_id: user.to_string(),
            collect_id: collect.to_string(),
            payload: json!({ "note": note }),
            base_version: 3,
            updated_at_ms: 0,
        }
    }

    #[test]
    fn drafts_round_trip_per_user_and_school() {
        let directory = test_directory("round-trip");
        let path = directory.join(DRAFTS_FILE);

        let saved = save_draft(
            &path,
            &draft("school-a", "user-1", "collect-1", "첫 초안"),
            10,
        )
        .expect("save");
        assert_eq!(saved.updated_at_ms, 10);
        save_draft(&path, &draft("school-a", "user-1", "collect-2", "둘째"), 20).expect("save");
        save_draft(
            &path,
            &draft("school-b", "user-1", "collect-3", "다른 학교"),
            30,
        )
        .expect("save");
        save_draft(
            &path,
            &draft("school-a", "user-2", "collect-1", "다른 사용자"),
            40,
        )
        .expect("save");

        let mine = list_drafts(&path, "school-a", "user-1").expect("list");
        assert_eq!(mine.len(), 2, "only this user's drafts in this school");
        assert_eq!(mine[0].collect_id, "collect-2", "newest first");

        // Saving again replaces the body instead of adding a row.
        save_draft(&path, &draft("school-a", "user-1", "collect-1", "수정"), 50).expect("save");
        let updated = find_draft(&path, "school-a", "user-1", "collect-1")
            .expect("find")
            .expect("draft");
        assert_eq!(updated.payload["note"], "수정");
        assert_eq!(draft_count(&path).expect("count"), 4);

        std::fs::remove_dir_all(&directory).expect("cleanup");
    }

    #[test]
    fn drafts_are_cleared_on_sign_out() {
        let directory = test_directory("sign-out");
        let path = directory.join(DRAFTS_FILE);

        save_draft(&path, &draft("school-a", "user-1", "collect-1", "가"), 10).expect("save");
        save_draft(&path, &draft("school-a", "user-1", "collect-2", "나"), 20).expect("save");
        save_draft(&path, &draft("school-a", "user-2", "collect-1", "다"), 30).expect("save");

        delete_draft(&path, "school-a", "user-1", "collect-1").expect("delete");
        assert!(
            find_draft(&path, "school-a", "user-1", "collect-1")
                .expect("find")
                .is_none()
        );

        let removed = clear_tenant_drafts(&path, "school-a", "user-1").expect("clear tenant");
        assert_eq!(removed, 1);
        assert_eq!(
            draft_count(&path).expect("count"),
            1,
            "another user keeps their draft"
        );

        let removed = clear_user_drafts(&path, "user-2").expect("clear user");
        assert_eq!(removed, 1);
        assert_eq!(draft_count(&path).expect("count"), 0);

        std::fs::remove_dir_all(&directory).expect("cleanup");
    }

    #[test]
    fn oversized_or_malformed_drafts_are_rejected() {
        let directory = test_directory("reject");
        let path = directory.join(DRAFTS_FILE);

        let mut empty_id = draft("school-a", "user-1", "collect-1", "가");
        empty_id.collect_id = "  ".to_string();
        assert!(save_draft(&path, &empty_id, 10).is_err());

        let mut broken_id = draft("school-a", "user-1", "collect-1", "가");
        broken_id.user_id = "user\n1".to_string();
        assert!(save_draft(&path, &broken_id, 10).is_err());

        let mut oversized = draft("school-a", "user-1", "collect-1", "가");
        oversized.payload = json!({ "note": "가".repeat(super::MAX_DRAFT_BYTES) });
        assert!(save_draft(&path, &oversized, 10).is_err());

        let mut negative = draft("school-a", "user-1", "collect-1", "가");
        negative.base_version = -1;
        assert!(save_draft(&path, &negative, 10).is_err());

        assert_eq!(draft_count(&path).expect("count"), 0);

        std::fs::remove_dir_all(&directory).expect("cleanup");
    }
}
