use std::env;

use anyhow::Context;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    school_collect_observability::init("migrator");

    // The schema owner is its own input on purpose: a runtime credential must
    // never be able to run DDL by being the only URL in the environment.
    let database_url = env::var("MIGRATION_DATABASE_URL")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .context("MIGRATION_DATABASE_URL is required for migrations")?;
    let pool = school_collect_db::connect(&database_url).await?;

    school_collect_db::MIGRATOR
        .run(&pool)
        .await
        .context("database migration failed")?;

    Ok(())
}
