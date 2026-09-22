use std::env;

use anyhow::Context;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    school_collect_observability::init("migrator");

    let database_url =
        env::var("DATABASE_URL").context("DATABASE_URL is required for migrations")?;
    let pool = school_collect_db::connect(&database_url).await?;

    school_collect_db::MIGRATOR
        .run(&pool)
        .await
        .context("database migration failed")?;

    Ok(())
}
