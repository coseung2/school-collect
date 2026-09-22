#[tokio::main]
async fn main() -> anyhow::Result<()> {
    school_collect_observability::init("worker");

    tracing::info!("worker foundation started; NATS JetStream wiring is Stage 6");
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("worker stopped");

    Ok(())
}
