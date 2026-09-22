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
