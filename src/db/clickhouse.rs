use chrono::{DateTime, Utc};
use clickhouse::{Client, Row};
use serde::Serialize;

use crate::db::writer::{PriceChangeRow, TickSizeRow, TradeRow};

pub struct Config {
    pub url: String,
    pub database: String,
    pub user: String,
    pub password: String,
}

impl Config {
    pub fn from_env() -> Result<Self, std::env::VarError> {
        Ok(Self {
            url: std::env::var("CLICKHOUSE_URL")?,
            database: std::env::var("CLICKHOUSE_DB")?,
            user: std::env::var("CLICKHOUSE_USER")?,
            password: std::env::var("CLICKHOUSE_PASSWORD")?,
        })
    }
}

pub fn connect(cfg: &Config) -> Client {
    Client::default()
        .with_url(&cfg.url)
        .with_database(&cfg.database)
        .with_user(&cfg.user)
        .with_password(&cfg.password)
        .with_setting("async_insert", "1")
        .with_setting("wait_for_async_insert", "0")
}

pub async fn migrate(client: &Client) -> Result<(), clickhouse::error::Error> {
    const DDL: &str = include_str!("../../schema_clickhouse.sql");

    for stmt in DDL.split(';') {
        let stmt = stmt.trim();
        if stmt.is_empty() || stmt.lines().all(|l| l.trim_start().starts_with("--")) {
            continue;
        }
        client.query(stmt).execute().await?;
    }
    Ok(())
}

#[derive(Debug, Row, Serialize)]
struct ChTrade<'a> {
    asset_id: &'a str,
    market: &'a str,
    #[serde(with = "clickhouse::serde::chrono::datetime64::millis")]
    ts: DateTime<Utc>,
    price: f64,
    size: Option<f64>,
    side: &'a str,
    transaction_hash: &'a str,
}

#[derive(Debug, Row, Serialize)]
struct ChPriceChange<'a> {
    asset_id: &'a str,
    market: &'a str,
    #[serde(with = "clickhouse::serde::chrono::datetime64::millis")]
    ts: DateTime<Utc>,
    price: f64,
    size: f64,
    side: &'a str,
    best_ask: Option<f64>,
    hash: &'a str,
}

#[derive(Debug, Row, Serialize)]
struct ChTickSize<'a> {
    asset_id: &'a str,
    market: &'a str,
    #[serde(with = "clickhouse::serde::chrono::datetime64::millis")]
    ts: DateTime<Utc>,
    old_tick_size: Option<f64>,
    new_tick_size: f64,
}

pub async fn insert_trades(
    client: &Client,
    rows: &[TradeRow],
) -> Result<(), clickhouse::error::Error> {
    if rows.is_empty() {
        return Ok(());
    }

    let mut insert = client.insert::<ChTrade>("trades").await?;
    for r in rows {
        insert
            .write(&ChTrade {
                asset_id: &r.asset_id,
                market: &r.market,
                ts: r.ts,
                price: r.price,
                size: r.size,
                side: &r.side,
                transaction_hash: r.transaction_hash.as_deref().unwrap_or(""),
            })
            .await?;
    }
    insert.end().await
}

pub async fn insert_price_changes(
    client: &Client,
    rows: &[PriceChangeRow],
) -> Result<(), clickhouse::error::Error> {
    if rows.is_empty() {
        return Ok(());
    }

    let mut insert = client.insert::<ChPriceChange>("price_changes").await?;
    for r in rows {
        insert
            .write(&ChPriceChange {
                asset_id: &r.asset_id,
                market: &r.market,
                ts: r.ts,
                price: r.price,
                size: r.size,
                side: &r.side,
                best_ask: r.best_ask,
                hash: r.hash.as_deref().unwrap_or(""),
            })
            .await?;
    }
    insert.end().await
}

pub async fn insert_tick_sizes(
    client: &Client,
    rows: &[TickSizeRow],
) -> Result<(), clickhouse::error::Error> {
    if rows.is_empty() {
        return Ok(());
    }

    let mut insert = client.insert::<ChTickSize>("tick_size_changes").await?;
    for r in rows {
        insert
            .write(&ChTickSize {
                asset_id: &r.asset_id,
                market: &r.market,
                ts: r.ts,
                old_tick_size: r.old_tick_size,
                new_tick_size: r.new_tick_size,
            })
            .await?;
    }
    insert.end().await
}
