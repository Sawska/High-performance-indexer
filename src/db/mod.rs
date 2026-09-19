pub mod writer;

use sqlx::postgres::{PgPool, PgPoolOptions};

/// Open the connection pool. `url` is a normal Postgres URL, e.g.
/// `postgres://user:pass@localhost/polymarket`.
pub async fn connect(url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new().max_connections(8).connect(url).await
}
