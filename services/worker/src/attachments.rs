//! Retention sweep for attachments.
//!
//! An attachment lives until its deadline. The sweep removes the bytes first and
//! only then the row, so a failed deletion is retried on the next pass instead of
//! leaving an object nobody can reach.

use std::{future::Future, sync::Arc, time::Duration};

use chrono::Utc;
use school_collect_application::storage::ObjectStorage;
use school_collect_db::{
    expired_attachments, orphaned_attachments, purge_attachment, purge_attachment_orphan,
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
    /// Objects whose record was kept because the attachment they belong to
    /// still exists, so a writer that resumes late is still covered.
    pub orphans_kept: usize,
    /// Attachments or objects the storage refused; they stay for the next pass.
    pub failed: usize,
}

impl PurgeReport {
    /// Whether the round had nothing to do at all.
    ///
    /// A round that only touched orphan records still counts as work: skipping
    /// it would silence a removal that failed and was left for the next pass.
    pub fn is_idle(&self) -> bool {
        self.considered == 0
            && self.purged == 0
            && self.orphans_removed == 0
            && self.orphans_kept == 0
            && self.failed == 0
    }
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

    // Bytes an attachment row no longer points at: a taken-over attempt's
    // object, or one whose cleanup delete failed. Age is not proof that the
    // writer that owned the key stopped, so the object is deleted again on
    // every pass and the record is retired only once the slot it came from is
    // gone: nothing is dropped while a late write could still recreate it.
    let orphans = orphaned_attachments(pool, batch).await?;
    for orphan in orphans {
        match storage.delete(&orphan.object_key).await {
            Ok(()) => {
                if orphan.attachment_missing {
                    purge_attachment_orphan(pool, orphan.id).await?;
                    report.orphans_removed += 1;
                    tracing::info!(
                        orphan.id = %orphan.id,
                        tenant.id = %orphan.tenant_id,
                        "untracked attachment bytes removed"
                    );
                } else {
                    report.orphans_kept += 1;
                }
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
            // Only a round with nothing to do at all stays quiet. Everything
            // else is reported, so orphan work and its failures cannot vanish
            // from the logs just because no attachment expired this time.
            Ok(report) if report.failed > 0 => {
                tracing::warn!(
                    considered = report.considered,
                    purged = report.purged,
                    orphans_removed = report.orphans_removed,
                    orphans_kept = report.orphans_kept,
                    failed = report.failed,
                    "attachment sweep finished with failures; those items stay for the next pass"
                );
            }
            Ok(report) if !report.is_idle() => {
                tracing::info!(
                    considered = report.considered,
                    purged = report.purged,
                    orphans_removed = report.orphans_removed,
                    orphans_kept = report.orphans_kept,
                    failed = report.failed,
                    "attachment sweep finished"
                );
            }
            // Nothing expired and no orphan record needed attention.
            Ok(_) => {}
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
    use super::{PurgeReport, run_every};
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    #[test]
    fn a_round_is_idle_only_when_it_had_nothing_to_do() {
        assert!(PurgeReport::default().is_idle());
        assert!(
            !PurgeReport {
                considered: 1,
                ..PurgeReport::default()
            }
            .is_idle()
        );
        assert!(
            !PurgeReport {
                orphans_removed: 1,
                ..PurgeReport::default()
            }
            .is_idle()
        );
        assert!(
            !PurgeReport {
                orphans_kept: 1,
                ..PurgeReport::default()
            }
            .is_idle()
        );
        assert!(
            !PurgeReport {
                failed: 1,
                ..PurgeReport::default()
            }
            .is_idle()
        );
    }

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
