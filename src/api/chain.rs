//! Read endpoints over the on-chain tables.
//!
//! These mirror the REST-backed endpoints deliberately: `/api/chain/fills` is
//! the settled counterpart to `/api/trades`, so the two can be diffed to find
//! trades the backend reported but the chain never settled (or vice versa).

use axum::Json;
use axum::extract::{Query, State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;

use super::ApiResult;
use crate::chain;

fn page_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(50).clamp(1, 500)
}

#[derive(Deserialize)]
pub struct FeedQuery {
    token_id: Option<String>,
    condition_id: Option<String>,
    wallet: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

impl FeedQuery {
    /// Addresses and hashes are stored lower-case, so callers can paste a
    /// checksummed address from a block explorer and still match.
    fn wallet(&self) -> String {
        self.wallet.clone().unwrap_or_default().to_lowercase()
    }

    fn condition(&self) -> String {
        self.condition_id.clone().unwrap_or_default().to_lowercase()
    }

    fn token(&self) -> String {
        self.token_id.clone().unwrap_or_default()
    }

    fn limit(&self) -> i64 {
        page_limit(self.limit)
    }

    fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}

// ── status ──────────────────────────────────────────────────────────────────

#[derive(Serialize, sqlx::FromRow)]
pub struct ChainTableCount {
    name: String,
    rows: i64,
}

#[derive(Serialize)]
pub struct ChainStatus {
    #[serde(flatten)]
    indexer: chain::Status,
    stored_cursor: Option<i64>,
    stored_cursor_at: Option<DateTime<Utc>>,
    tables: Vec<ChainTableCount>,
}

pub async fn status(State(pool): State<PgPool>) -> ApiResult<ChainStatus> {
    let cursor: Option<(i64, DateTime<Utc>)> =
        sqlx::query_as("SELECT last_block, updated_at FROM chain_cursors WHERE name = 'polygon'")
            .fetch_optional(&pool)
            .await?;

    let tables = sqlx::query_as::<_, ChainTableCount>(
        "SELECT relname::text AS name, n_live_tup AS rows
            FROM pg_stat_user_tables
           WHERE relname LIKE 'chain\\_%'
        ORDER BY relname",
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(ChainStatus {
        indexer: chain::status(),
        stored_cursor: cursor.map(|c| c.0),
        stored_cursor_at: cursor.map(|c| c.1),
        tables,
    }))
}

// ── fills ───────────────────────────────────────────────────────────────────

#[derive(Serialize, sqlx::FromRow)]
pub struct FillRow {
    transaction_hash: String,
    log_index: i64,
    block_number: i64,
    block_time: Option<DateTime<Utc>>,
    exchange: String,
    exchange_version: i32,
    neg_risk: bool,
    event: String,
    order_hash: String,
    maker: Option<String>,
    taker: Option<String>,
    side: Option<String>,
    token_id: Option<String>,
    price: Option<f64>,
    size: Option<f64>,
    fee: Option<f64>,
    /// From the catalogue join; lets the UI link a fill to its market page.
    condition_id: Option<String>,
    question: Option<String>,
    slug: Option<String>,
}

pub async fn fills(
    State(pool): State<PgPool>,
    Query(q): Query<FeedQuery>,
) -> ApiResult<Vec<FillRow>> {
    // `market_tokens` is the indexed token -> market mapping; joining straight
    // against `markets.clob_token_ids` would scan the whole catalogue per row.
    let rows = sqlx::query_as::<_, FillRow>(
        "SELECT f.transaction_hash, f.log_index, f.block_number, f.block_time,
                f.exchange, f.exchange_version, f.neg_risk, f.event, f.order_hash,
                f.maker, f.taker, f.side, f.token_id,
                f.price::float8 AS price, f.size::float8 AS size, f.fee::float8 AS fee,
                m.condition_id, m.question, m.slug
           FROM chain_order_fills f
           LEFT JOIN market_tokens mt ON mt.token_id = f.token_id
           LEFT JOIN markets m ON m.id = mt.market_id
          WHERE ($1 = '' OR f.token_id = $1)
            AND ($2 = '' OR f.maker = $2 OR f.taker = $2)
          ORDER BY f.block_number DESC, f.log_index DESC
          LIMIT $3 OFFSET $4",
    )
    .bind(q.token())
    .bind(q.wallet())
    .bind(q.limit())
    .bind(q.offset())
    .fetch_all(&pool)
    .await?;

    Ok(Json(rows))
}

// ── splits / merges ─────────────────────────────────────────────────────────

#[derive(Serialize, sqlx::FromRow)]
pub struct PositionRow {
    transaction_hash: String,
    log_index: i64,
    block_number: i64,
    block_time: Option<DateTime<Utc>>,
    kind: String,
    stakeholder: String,
    condition_id: String,
    collateral_token: Option<String>,
    partition_ids: Value,
    amount: Option<f64>,
    question: Option<String>,
}

pub async fn positions(
    State(pool): State<PgPool>,
    Query(q): Query<FeedQuery>,
) -> ApiResult<Vec<PositionRow>> {
    let rows = sqlx::query_as::<_, PositionRow>(
        "SELECT p.transaction_hash, p.log_index, p.block_number, p.block_time, p.kind,
                p.stakeholder, p.condition_id, p.collateral_token, p.partition_ids,
                p.amount::float8 AS amount, m.question
           FROM chain_ctf_positions p
           LEFT JOIN markets m ON m.condition_id = p.condition_id
          WHERE ($1 = '' OR p.condition_id = $1)
            AND ($2 = '' OR p.stakeholder = $2)
          ORDER BY p.block_number DESC, p.log_index DESC
          LIMIT $3 OFFSET $4",
    )
    .bind(q.condition())
    .bind(q.wallet())
    .bind(q.limit())
    .bind(q.offset())
    .fetch_all(&pool)
    .await?;

    Ok(Json(rows))
}

// ── redemptions ─────────────────────────────────────────────────────────────

#[derive(Serialize, sqlx::FromRow)]
pub struct RedemptionRow {
    transaction_hash: String,
    log_index: i64,
    block_number: i64,
    block_time: Option<DateTime<Utc>>,
    redeemer: String,
    condition_id: String,
    index_sets: Value,
    payout: Option<f64>,
    question: Option<String>,
}

pub async fn redemptions(
    State(pool): State<PgPool>,
    Query(q): Query<FeedQuery>,
) -> ApiResult<Vec<RedemptionRow>> {
    let rows = sqlx::query_as::<_, RedemptionRow>(
        "SELECT r.transaction_hash, r.log_index, r.block_number, r.block_time, r.redeemer,
                r.condition_id, r.index_sets, r.payout::float8 AS payout, m.question
           FROM chain_ctf_redemptions r
           LEFT JOIN markets m ON m.condition_id = r.condition_id
          WHERE ($1 = '' OR r.condition_id = $1)
            AND ($2 = '' OR r.redeemer = $2)
          ORDER BY r.block_number DESC, r.log_index DESC
          LIMIT $3 OFFSET $4",
    )
    .bind(q.condition())
    .bind(q.wallet())
    .bind(q.limit())
    .bind(q.offset())
    .fetch_all(&pool)
    .await?;

    Ok(Json(rows))
}

// ── conditions ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ConditionsQuery {
    condition_id: Option<String>,
    /// `true` for resolved only, `false` for still-open only, absent for both.
    resolved: Option<bool>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ConditionRow {
    condition_id: String,
    oracle: Option<String>,
    question_id: Option<String>,
    outcome_slot_count: Option<i64>,
    prepared_at: Option<DateTime<Utc>>,
    resolved_at: Option<DateTime<Utc>>,
    payout_numerators: Option<Value>,
    question: Option<String>,
    slug: Option<String>,
}

pub async fn conditions(
    State(pool): State<PgPool>,
    Query(q): Query<ConditionsQuery>,
) -> ApiResult<Vec<ConditionRow>> {
    let rows = sqlx::query_as::<_, ConditionRow>(
        "SELECT c.condition_id, c.oracle, c.question_id, c.outcome_slot_count,
                c.prepared_at, c.resolved_at, c.payout_numerators, m.question, m.slug
           FROM chain_conditions c
           LEFT JOIN markets m ON m.condition_id = c.condition_id
          WHERE ($1 = '' OR c.condition_id = $1)
            AND ($2::bool IS NULL
                 OR ($2 AND c.resolved_at IS NOT NULL)
                 OR (NOT $2 AND c.resolved_at IS NULL))
          ORDER BY COALESCE(c.resolved_at, c.prepared_at) DESC NULLS LAST
          LIMIT $3 OFFSET $4",
    )
    .bind(q.condition_id.unwrap_or_default().to_lowercase())
    .bind(q.resolved)
    .bind(page_limit(q.limit))
    .bind(q.offset.unwrap_or(0).max(0))
    .fetch_all(&pool)
    .await?;

    Ok(Json(rows))
}

// ── transfers ───────────────────────────────────────────────────────────────

#[derive(Serialize, sqlx::FromRow)]
pub struct TokenTransferRow {
    transaction_hash: String,
    log_index: i64,
    item_index: i32,
    block_number: i64,
    block_time: Option<DateTime<Utc>>,
    operator: Option<String>,
    from_address: String,
    to_address: String,
    token_id: String,
    amount: Option<f64>,
}

pub async fn transfers(
    State(pool): State<PgPool>,
    Query(q): Query<FeedQuery>,
) -> ApiResult<Vec<TokenTransferRow>> {
    let rows = sqlx::query_as::<_, TokenTransferRow>(
        "SELECT transaction_hash, log_index, item_index, block_number, block_time,
                operator, from_address, to_address, token_id, amount::float8 AS amount
           FROM chain_token_transfers
          WHERE ($1 = '' OR token_id = $1)
            AND ($2 = '' OR from_address = $2 OR to_address = $2)
          ORDER BY block_number DESC, log_index DESC, item_index
          LIMIT $3 OFFSET $4",
    )
    .bind(q.token())
    .bind(q.wallet())
    .bind(q.limit())
    .bind(q.offset())
    .fetch_all(&pool)
    .await?;

    Ok(Json(rows))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct CollateralTransferRow {
    transaction_hash: String,
    log_index: i64,
    block_number: i64,
    block_time: Option<DateTime<Utc>>,
    token: String,
    symbol: Option<String>,
    from_address: String,
    to_address: String,
    amount: Option<f64>,
}

pub async fn collateral(
    State(pool): State<PgPool>,
    Query(q): Query<FeedQuery>,
) -> ApiResult<Vec<CollateralTransferRow>> {
    let rows = sqlx::query_as::<_, CollateralTransferRow>(
        "SELECT transaction_hash, log_index, block_number, block_time, token, symbol,
                from_address, to_address, amount::float8 AS amount
           FROM chain_collateral_transfers
          WHERE ($1 = '' OR from_address = $1 OR to_address = $1)
          ORDER BY block_number DESC, log_index DESC
          LIMIT $2 OFFSET $3",
    )
    .bind(q.wallet())
    .bind(q.limit())
    .bind(q.offset())
    .fetch_all(&pool)
    .await?;

    Ok(Json(rows))
}

#[cfg(test)]
mod unit {
    use super::*;

    fn query(wallet: Option<&str>, limit: Option<i64>, offset: Option<i64>) -> FeedQuery {
        FeedQuery {
            token_id: None,
            condition_id: Some("0xABCDEF".into()),
            wallet: wallet.map(str::to_string),
            limit,
            offset,
        }
    }

    #[test]
    fn identifiers_are_lower_cased_for_matching() {
        let q = query(Some("0xAbCdEf1234567890AbCdEf1234567890AbCdEf12"), None, None);
        assert_eq!(q.wallet(), "0xabcdef1234567890abcdef1234567890abcdef12");
        assert_eq!(q.condition(), "0xabcdef");
    }

    #[test]
    fn an_absent_filter_becomes_the_empty_sentinel() {
        let q = query(None, None, None);
        assert_eq!(q.wallet(), "", "empty means 'no filter' in the SQL");
        assert_eq!(q.token(), "");
    }

    #[test]
    fn paging_is_clamped() {
        assert_eq!(query(None, Some(10_000), None).limit(), 500);
        assert_eq!(query(None, Some(0), None).limit(), 1);
        assert_eq!(query(None, None, None).limit(), 50);
        assert_eq!(query(None, None, Some(-5)).offset(), 0);
    }

    #[test]
    fn token_ids_keep_their_case_and_length() {
        // Token ids are decimal, not hex: lower-casing would be meaningless and
        // truncating would break the join.
        let q = FeedQuery {
            token_id: Some("7132".repeat(19)),
            condition_id: None,
            wallet: None,
            limit: None,
            offset: None,
        };
        assert_eq!(q.token().len(), 76);
    }
}
