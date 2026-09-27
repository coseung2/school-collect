//! Attachment policy.
//!
//! The rule set is deliberately small and explicit: what may be stored, how
//! much, how long, and who may read or remove it. Anything the API cannot
//! decide from these functions must be decided by a manager.

use chrono::{DateTime, Duration, Utc};

use crate::MembershipRole;

/// Largest single attachment the service accepts.
pub const MAX_ATTACHMENT_BYTES: i64 = 10 * 1024 * 1024;
/// Attachments one submission item may hold.
pub const MAX_ATTACHMENTS_PER_ITEM: i64 = 5;
/// Attachments one submission may hold across all of its items.
pub const MAX_ATTACHMENTS_PER_SUBMISSION: i64 = 20;
/// How long an attachment stays available after it is created.
pub const ATTACHMENT_RETENTION_DAYS: i64 = 180;
/// Longest accepted file name.
pub const MAX_FILE_NAME_LEN: usize = 120;
/// Longest accepted content type.
pub const MAX_CONTENT_TYPE_LEN: usize = 120;

/// Content types the service stores. School paper arrives as documents,
/// spreadsheets, presentations, images, plain text, or a ZIP of those.
pub const ALLOWED_CONTENT_TYPES: &[&str] = &[
    "application/pdf",
    "application/x-hwp",
    "application/haansofthwp",
    "application/vnd.hancom.hwp",
    "application/vnd.hancom.hwpx",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    "application/vnd.ms-excel",
    "application/vnd.ms-powerpoint",
    "application/msword",
    "image/png",
    "image/jpeg",
    "text/plain",
    "application/zip",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentRejection {
    EmptyFile,
    TooLarge,
    UnsupportedContentType,
    InvalidFileName,
}

impl AttachmentRejection {
    pub const fn code(self) -> &'static str {
        match self {
            Self::EmptyFile => "attachment_empty",
            Self::TooLarge => "attachment_too_large",
            Self::UnsupportedContentType => "attachment_type_not_allowed",
            Self::InvalidFileName => "attachment_name_invalid",
        }
    }

    pub const fn message(self) -> &'static str {
        match self {
            Self::EmptyFile => "첨부 파일이 비어 있습니다.",
            Self::TooLarge => "첨부 파일이 너무 큽니다.",
            Self::UnsupportedContentType => "지원하지 않는 파일 형식입니다.",
            Self::InvalidFileName => "파일 이름을 사용할 수 없습니다.",
        }
    }
}

/// Checks one new attachment and returns the file name to store.
///
/// The name is a label only: object keys are generated from identifiers, so a
/// name can never point at another school's object.
pub fn validate_new_attachment(
    file_name: &str,
    content_type: &str,
    byte_size: i64,
) -> Result<String, AttachmentRejection> {
    let file_name = file_name.trim();
    if file_name.is_empty()
        || file_name.chars().count() > MAX_FILE_NAME_LEN
        || file_name.contains(['/', '\\', '\t', '\r', '\n'])
        || file_name.contains("..")
        || file_name.starts_with('.')
    {
        return Err(AttachmentRejection::InvalidFileName);
    }

    let content_type = content_type.trim().to_ascii_lowercase();
    if content_type.is_empty()
        || content_type.chars().count() > MAX_CONTENT_TYPE_LEN
        || !ALLOWED_CONTENT_TYPES.contains(&content_type.as_str())
    {
        return Err(AttachmentRejection::UnsupportedContentType);
    }

    if byte_size <= 0 {
        return Err(AttachmentRejection::EmptyFile);
    }
    if byte_size > MAX_ATTACHMENT_BYTES {
        return Err(AttachmentRejection::TooLarge);
    }

    Ok(file_name.to_owned())
}

/// Object key for one attachment. Tenant first, so a bucket listing is scoped.
pub fn object_key(tenant_id: &str, attachment_id: &str) -> String {
    format!("tenants/{tenant_id}/attachments/{attachment_id}")
}

/// Object key for one upload attempt of an attachment.
///
/// Every attempt writes its own object, so a writer whose claim was taken over
/// cannot overwrite the bytes of the attempt that replaced it. The attachment
/// key alone is a prefix that is never written to.
pub fn object_key_for_attempt(tenant_id: &str, attachment_id: &str, attempt_id: &str) -> String {
    format!("{}/{attempt_id}", object_key(tenant_id, attachment_id))
}

/// Deadline after which the bytes are removed and the row is purged.
pub fn expires_at(created_at: DateTime<Utc>) -> DateTime<Utc> {
    created_at + Duration::days(ATTACHMENT_RETENTION_DAYS)
}

/// Who may read an attachment: its owner, or a manager of the school.
pub const fn can_read(role: MembershipRole, owner: bool) -> bool {
    role.can_manage_collects() || (owner && role.can_submit())
}

/// Who may remove an attachment before the retention sweep does.
///
/// A manager can always remove one. The owner can only remove it while their
/// submission is still open; an answer that was handed in keeps its files.
pub const fn can_delete(role: MembershipRole, owner: bool, submission_submitted: bool) -> bool {
    role.can_manage_collects() || (owner && !submission_submitted)
}

#[cfg(test)]
mod tests {
    use super::{
        ALLOWED_CONTENT_TYPES, ATTACHMENT_RETENTION_DAYS, AttachmentRejection,
        MAX_ATTACHMENT_BYTES, can_delete, can_read, expires_at, object_key, object_key_for_attempt,
        validate_new_attachment,
    };
    use crate::MembershipRole;
    use chrono::{TimeZone, Utc};

    #[test]
    fn accepts_a_school_document() {
        assert_eq!(
            validate_new_attachment("계획서.pdf", "application/pdf", 2048),
            Ok("계획서.pdf".to_owned())
        );
        assert_eq!(
            validate_new_attachment("  사진.png  ", "IMAGE/PNG", 1),
            Ok("사진.png".to_owned())
        );
    }

    #[test]
    fn rejects_names_that_could_escape_the_object_namespace() {
        for name in [
            "",
            "   ",
            "../escape.pdf",
            "folder/file.pdf",
            "folder\\file.pdf",
            ".hidden.pdf",
            "line\nbreak.pdf",
            &"a".repeat(super::MAX_FILE_NAME_LEN + 1),
        ] {
            assert_eq!(
                validate_new_attachment(name, "application/pdf", 10),
                Err(AttachmentRejection::InvalidFileName),
                "{name:?} must be rejected"
            );
        }
    }

    #[test]
    fn rejects_size_and_type_outside_the_policy() {
        assert_eq!(
            validate_new_attachment("empty.pdf", "application/pdf", 0),
            Err(AttachmentRejection::EmptyFile)
        );
        assert_eq!(
            validate_new_attachment("big.pdf", "application/pdf", MAX_ATTACHMENT_BYTES + 1),
            Err(AttachmentRejection::TooLarge)
        );
        assert_eq!(
            validate_new_attachment("script.exe", "application/x-msdownload", 10),
            Err(AttachmentRejection::UnsupportedContentType)
        );
        assert_eq!(
            validate_new_attachment("plain.pdf", "", 10),
            Err(AttachmentRejection::UnsupportedContentType)
        );
    }

    #[test]
    fn policy_lists_hancom_and_office_documents() {
        assert!(ALLOWED_CONTENT_TYPES.contains(&"application/x-hwp"));
        assert!(ALLOWED_CONTENT_TYPES.contains(&"application/vnd.hancom.hwpx"));
    }

    #[test]
    fn object_keys_and_expiry_follow_the_tenant_and_retention() {
        assert_eq!(
            object_key("tenant-1", "attachment-1"),
            "tenants/tenant-1/attachments/attachment-1"
        );
        // Each attempt writes a distinct object under the same attachment.
        assert_eq!(
            object_key_for_attempt("tenant-1", "attachment-1", "attempt-a"),
            "tenants/tenant-1/attachments/attachment-1/attempt-a"
        );
        assert_ne!(
            object_key_for_attempt("tenant-1", "attachment-1", "attempt-a"),
            object_key_for_attempt("tenant-1", "attachment-1", "attempt-b")
        );
        let created = Utc.with_ymd_and_hms(2026, 9, 27, 0, 0, 0).unwrap();
        assert_eq!(
            expires_at(created),
            created + chrono::Duration::days(ATTACHMENT_RETENTION_DAYS)
        );
    }

    #[test]
    fn readers_are_the_owner_or_a_manager() {
        assert!(can_read(MembershipRole::Admin, false));
        assert!(can_read(MembershipRole::Coordinator, false));
        assert!(can_read(MembershipRole::Contributor, true));
        assert!(!can_read(MembershipRole::Contributor, false));
        assert!(!can_read(MembershipRole::Viewer, true));
        assert!(!can_read(MembershipRole::Viewer, false));
    }

    #[test]
    fn removal_is_open_to_managers_and_to_an_unsubmitted_owner() {
        assert!(can_delete(MembershipRole::Admin, false, true));
        assert!(can_delete(MembershipRole::Contributor, true, false));
        assert!(!can_delete(MembershipRole::Contributor, true, true));
        assert!(!can_delete(MembershipRole::Viewer, false, false));
    }
}
