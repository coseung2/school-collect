//! Retention sweep for attachments.
//!
//! An attachment lives until its deadline. The sweep removes the bytes first and
//! only then the row, so a failed deletion is retried on the next pass instead of
//! leaving an object nobody can reach.

use std::{future::Future, sync::Arc, time::Duration};

use chrono::Utc;
use school_collect_application::storage::ObjectStorage;
use school_collect_db::{expired_attachments, purge_attachment};
use sqlx::PgPool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PurgeReport {
    /// Attachments whose deadline had passed.
    pub considered: usize,
    /// Attachments whose bytes and row are gone.
    pub purged: usize,
    /// Attachments the storage refused; they stay for the next pass.
    pub failed: usize,
}

/// Removes one batch of expired attachments.
pub async fn purge_expired_attachments(
    pool: &PgPool,
    storage: &dyn ObjectStorage,
    batch: i64,
) -> anyhow::Result<PurgeReport> {
    let expired = expired_attachments(pool, Utc::now(), batch).await?;
    let mut report = PurgeReport {
        considered: expired.len(),
        ..PurgeReport::default()
    };

    for attachment in expired {
        match storage.delete(&attachment.object_key).await {
            Ok(()) => {
                purge_attachment(pool, attachment.id).await?;
                report.purged += 1;
                tracing::info!(
                    attachment.id = %attachment.id,
                    tenant.id = %attachment.tenant_id,
                    "expired attachment removed"
                );
            }
            Err(error) => {
                report.failed += 1;
                tracing::warn!(
                    attachment.id = %attachment.id,
                    reason = %error,
                    "expired attachment could not be removed; retrying later"
                );
            }
        }
    }

    Ok(report)
}

/// Runs the sweep until the shutdown future resolves.
pub async fn run_sweep_until_shutdown(
    pool: PgPool,
    storage: Arc<dyn ObjectStorage>,
    batch: i64,
    interval: Duration,
    shutdown: impl Future<Output = ()>,
) {
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            () = &mut shutdown => {
                tracing::info!("attachment sweep stopping");
                return;
            }
            result = purge_expired_attachments(&pool, storage.as_ref(), batch) => {
                match result {
                    Ok(report) if report.considered == 0 => {}
                    Ok(report) => {
                        tracing::info!(
                            considered = report.considered,
                            purged = report.purged,
                            failed = report.failed,
                            "attachment sweep finished"
                        );
                    }
                    Err(error) => {
                        tracing::error!(%error, "attachment sweep round failed");
                    }
                }
            }
            () = tokio::time::sleep(interval) => {}
        }
    }
}
