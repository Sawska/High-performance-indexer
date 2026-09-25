use chrono::{DateTime,Utc};
use sqlx::PgPool;

#[derive(Debug,sqlx::FromRow)]
pub struct Trade {
    pub ts: DateTime<Utc>,
    pub price: f64,
    pub size: Option<f64>,
    pub side: String,
}

pub async fn recent_trades(pool: &PgPool, asset_id: &str, limit: i64) -> Result<Vec<Trade>, sqlx::Error> {
    sqlx::query_as::<_, Trade>(
        "SELECT ts, price::float8 AS price, size::float8 AS size, side
            FROM trades
        WHERE asset_id = $1
        ORDER BY ts DESC
        LIMIT $2",
    )
    .bind(asset_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn current_tick_size(pool: &PgPool, asset_id: &str) -> Result<Option<f64>, sqlx::Error> {
    sqlx::query_scalar::<_, f64>(
        "SELECT new_tick_size::float8
            FROM tick_size_changes
        WHERE asset_id = $1
        ORDER BY ts DESC
        LIMIT 1",
    )
    .bind(asset_id)
    .fetch_optional(pool)
    .await
}

#[derive(Debug,sqlx::FromRow)]
pub struct Candle {
    pub bucket: DateTime<Utc>,
    pub open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: Option<f64>,
}

pub async fn candles(pool: &PgPool, asset_id: &str, interval: &str, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<Vec<Candle>, sqlx::Error> {
    sqlx::query_as::<_, Candle>(
        "SELECT date_bucket($2::interval, ts) AS bucket
                (array_agg(price ORDER BY ts))[1]::float8
                max(price)::float8
                min(price)::float8
                (array_agg(price ORDER BY ts DESC))[1]::float8
                sum(size)::float8
            FROM trades
            WHERE asset_id = $1 AND ts >= $3 AND ts < $4
            GROUP BY bucket
            ORDER BY bucket",
    )
    .bind(asset_id)
    .bind(interval)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
}