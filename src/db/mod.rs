pub mod writer;
pub mod reader;
pub mod clickhouse;
pub mod rest_writer;

use sqlx::postgres::{PgPool, PgPoolOptions};

pub async fn connect(url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new().max_connections(8).connect(url).await
}
