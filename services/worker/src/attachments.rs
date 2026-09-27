//! Retention sweep for attachments.
//!
//! An attachment lives until its deadline. The sweep removes the bytes first and
//! only then the row, so a failed deletion is retried on the next pass instead of
//! leaving an object nobody can reach.

use std::{future::Future, sync::Arc, time::Duration};

use chrono::{Duration as ChronoDuration, Utc};
use school_collect_application::storage::ObjectStorage;
use school_collect_db::{
    UPLOAD_CLAIM_TIMEOUT_SECONDS, expired_attachments, orphaned_attachments, purge_attachment,
    purge_attachment_orphan,
};
use sqlx::PgPool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PurgeReport {
    /// Attachments whose deadline had passed.
    pub considered: usize,
    /// Attachments whose bytes and row are gone.
    pub purged: usize,
    /// Objects whose owning row was gone and whose bytes are now removed too.
    pub orphans_removed: usize,
    /// Attachments or objects the storage refused; they stay for the next pass.
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

    // Bytes whose owning row is gone: a taken-over attempt's object, or one
    // whose cleanup delete failed. They have no metadata left to age out, so
    // they are removed here instead of lingering untracked in the bucket. Only
    // objects recorded before the claim timeout are taken: a younger one may
    // still belong to a writer that is allowed to send its bytes, and clearing
    // the record while that is possible would lose it.
    let claim_cutoff = Utc::now() - ChronoDuration::seconds(UPLOAD_CLAIM_TIMEOUT_SECONDS);
    let orphans = orphaned_attachments(pool, claim_cutoff, batch).await?;
    for orphan in orphans {
        match storage.delete(&orphan.object_key).await {
            Ok(()) => {
                purge_attachment_orphan(pool, orphan.id).await?;
                report.orphans_removed += 1;
                tracing::info!(
                    orphan.id = %orphan.id,
                    tenant.id = %orphan.tenant_id,
                    "untracked attachment bytes removed"
                );
            }
            Err(error) => {
                report.failed += 1;
                tracing::warn!(
                    orphan.id = %orphan.id,
                    reason = %error,
                    "untracked attachment bytes could not be removed; retrying later"
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
    run_every(interval, shutdown, || async {
        match purge_expired_attachments(&pool, storage.as_ref(), batch).await {
            Ok(report) if report.considered == 0 => {}
            Ok(report) => {
                tracing::info!(
                    considered = report.considered,
                    purged = report.purged,
                    orphans_removed = report.orphans_removed,
                    failed = report.failed,
                    "attachment sweep finished"
                );
            }
            Err(error) => {
                tracing::error!(%error, "attachment sweep round failed");
            }
        }
    })
    .await;
    tracing::info!("attachment sweep stopping");
}

/// Runs `round`, then waits the full `interval`, until `shutdown` resolves.
///
/// The wait starts only after a round finishes and never races it, so a quick
/// round cannot trigger the next one immediately, and failures are retried at
/// the configured pace instead of in a tight loop.
async fn run_every<F, Fut>(interval: Duration, shutdown: impl Future<Output = ()>, mut round: F)
where
    F: FnMut() -> Fut,
    Fut: Future<Output = ()>,
{
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            () = &mut shutdown => return,
            () = round() => {}
        }
        tokio::select! {
            () = &mut shutdown => return,
            () = tokio::time::sleep(interval) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::run_every;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    #[tokio::test(start_paused = true)]
    async fn rounds_are_spaced_by_the_full_interval() {
        let rounds = Arc::new(AtomicUsize::new(0));
        let counter = rounds.clone();
        let interval = Duration::from_secs(6 * 60 * 60);
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let task = tokio::spawn(run_every(
            interval,
            async {
                let _ = stopped.await;
            },
            move || {
                let counter = counter.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                }
            },
        ));

        // The first round runs at once; an instant round must not start another.
        tokio::task::yield_now().await;
        assert_eq!(rounds.load(Ordering::SeqCst), 1);
        tokio::time::advance(interval - Duration::from_secs(1)).await;
        assert_eq!(
            rounds.load(Ordering::SeqCst),
            1,
            "no round before the interval ends"
        );
        tokio::time::advance(Duration::from_secs(1)).await;
        tokio::task::yield_now().await;
        assert_eq!(rounds.load(Ordering::SeqCst), 2);

        stop.send(()).expect("stop");
        task.await.expect("sweep stops on shutdown");
    }
}
