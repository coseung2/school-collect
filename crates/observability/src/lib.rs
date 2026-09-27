use std::sync::atomic::{AtomicU64, Ordering};

use tracing_subscriber::EnvFilter;

pub fn init(service_name: &'static str) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();

    tracing::info!(service.name = service_name, "observability initialized");
}

/// Minimal in-process counters for operations.
///
/// The registry is intentionally dependency-free: it covers the signals the
/// operational checklist needs (traffic, rejected access, storage failures) and
/// is rendered in the Prometheus text format. A full metrics client and
/// exporter remain a later, separately approved decision.
#[derive(Debug, Default)]
pub struct Metrics {
    http_requests_total: AtomicU64,
    http_responses_2xx: AtomicU64,
    http_responses_4xx: AtomicU64,
    http_responses_5xx: AtomicU64,
    auth_failures_total: AtomicU64,
    authorization_denials_total: AtomicU64,
    storage_failures_total: AtomicU64,
}

impl Metrics {
    pub fn global() -> &'static Metrics {
        static METRICS: Metrics = Metrics {
            http_requests_total: AtomicU64::new(0),
            http_responses_2xx: AtomicU64::new(0),
            http_responses_4xx: AtomicU64::new(0),
            http_responses_5xx: AtomicU64::new(0),
            auth_failures_total: AtomicU64::new(0),
            authorization_denials_total: AtomicU64::new(0),
            storage_failures_total: AtomicU64::new(0),
        };
        &METRICS
    }

    pub fn record_response(&self, status: u16) {
        self.http_requests_total.fetch_add(1, Ordering::Relaxed);
        let counter = match status {
            200..=299 => &self.http_responses_2xx,
            400..=499 => &self.http_responses_4xx,
            500..=599 => &self.http_responses_5xx,
            _ => return,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_auth_failure(&self) {
        self.auth_failures_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_authorization_denial(&self) {
        self.authorization_denials_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_storage_failure(&self) {
        self.storage_failures_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Prometheus text exposition of the current counters.
    pub fn render(&self) -> String {
        let mut output = String::new();
        for (name, help, value) in [
            (
                "school_collect_http_requests_total",
                "HTTP requests handled",
                self.http_requests_total.load(Ordering::Relaxed),
            ),
            (
                "school_collect_http_responses_2xx_total",
                "HTTP responses with a 2xx status",
                self.http_responses_2xx.load(Ordering::Relaxed),
            ),
            (
                "school_collect_http_responses_4xx_total",
                "HTTP responses with a 4xx status",
                self.http_responses_4xx.load(Ordering::Relaxed),
            ),
            (
                "school_collect_http_responses_5xx_total",
                "HTTP responses with a 5xx status",
                self.http_responses_5xx.load(Ordering::Relaxed),
            ),
            (
                "school_collect_auth_failures_total",
                "Requests rejected because the access token could not be verified",
                self.auth_failures_total.load(Ordering::Relaxed),
            ),
            (
                "school_collect_authorization_denials_total",
                "Requests rejected because the membership role does not allow the action",
                self.authorization_denials_total.load(Ordering::Relaxed),
            ),
            (
                "school_collect_storage_failures_total",
                "Requests that failed because the database could not complete the operation",
                self.storage_failures_total.load(Ordering::Relaxed),
            ),
        ] {
            output.push_str(&format!("# HELP {name} {help}\n"));
            output.push_str(&format!("# TYPE {name} counter\n"));
            output.push_str(&format!("{name} {value}\n"));
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::Metrics;

    #[test]
    fn counters_render_in_the_prometheus_text_format() {
        let metrics = Metrics::default();
        metrics.record_response(200);
        metrics.record_response(204);
        metrics.record_response(403);
        metrics.record_response(503);
        metrics.record_auth_failure();
        metrics.record_authorization_denial();
        metrics.record_storage_failure();

        let rendered = metrics.render();

        assert!(rendered.contains("# TYPE school_collect_http_requests_total counter"));
        assert!(rendered.contains("school_collect_http_requests_total 4"));
        assert!(rendered.contains("school_collect_http_responses_2xx_total 2"));
        assert!(rendered.contains("school_collect_http_responses_4xx_total 1"));
        assert!(rendered.contains("school_collect_http_responses_5xx_total 1"));
        assert!(rendered.contains("school_collect_auth_failures_total 1"));
        assert!(rendered.contains("school_collect_authorization_denials_total 1"));
        assert!(rendered.contains("school_collect_storage_failures_total 1"));
    }
}
