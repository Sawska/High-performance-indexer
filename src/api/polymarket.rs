

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use super::{ApiError, ApiResult};

const LIVE: &str = "NOT coalesce(closed, false) AND NOT coalesce(archived, false)";

fn page_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(50).clamp(1, 200)
}

fn json_list(raw: Option<&str>) -> Vec<String> {
    raw.and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
        .unwrap_or_default()
}

const MARKET_COLS: &str = "id, question, slug, condition_id, category, closed, end_date,
    volume_num::float8 AS volume, liquidity_num::float8 AS liquidity,
    best_bid::float8 AS best_bid, best_ask::float8 AS best_ask,
    last_trade_price::float8 AS last_trade_price, spread::float8 AS spread,
    outcomes, outcome_prices, clob_token_ids, scraped_at,
    raw->>'icon' AS icon, (raw->>'oneDayPriceChange')::float8 AS one_day_change";

#[derive(sqlx::FromRow)]
struct MarketRow {
    id: String,
    question: Option<String>,
    slug: Option<String>,
    condition_id: String,
    category: Option<String>,
    closed: Option<bool>,
    end_date: Option<String>,
    volume: Option<f64>,
    liquidity: Option<f64>,
    best_bid: Option<f64>,
    best_ask: Option<f64>,
    last_trade_price: Option<f64>,
    spread: Option<f64>,
    outcomes: Option<String>,
    outcome_prices: Option<String>,
    clob_token_ids: Option<String>,
    scraped_at: DateTime<Utc>,
    icon: Option<String>,
    one_day_change: Option<f64>,
}

#[derive(Serialize)]
pub struct Market {
    id: String,
    question: Option<String>,
    slug: Option<String>,
    condition_id: String,
    category: Option<String>,
    closed: bool,
    end_date: Option<String>,
    volume: Option<f64>,
    liquidity: Option<f64>,
    best_bid: Option<f64>,
    best_ask: Option<f64>,
    last_trade_price: Option<f64>,
    spread: Option<f64>,
    outcomes: Vec<String>,
    outcome_prices: Vec<f64>,
    token_ids: Vec<String>,
    scraped_at: DateTime<Utc>,
    icon: Option<String>,
    one_day_change: Option<f64>,
}

impl From<MarketRow> for Market {
    fn from(r: MarketRow) -> Self {
        Self {
            outcomes: json_list(r.outcomes.as_deref()),
            outcome_prices: json_list(r.outcome_prices.as_deref())
                .iter()
                .filter_map(|p| p.parse().ok())
                .collect(),
            token_ids: json_list(r.clob_token_ids.as_deref()),
            id: r.id,
            question: r.question,
            slug: r.slug,
            condition_id: r.condition_id,
            category: r.category,
            closed: r.closed.unwrap_or(false),
            end_date: r.end_date,
            volume: r.volume,
            liquidity: r.liquidity,
            best_bid: r.best_bid,
            best_ask: r.best_ask,
            last_trade_price: r.last_trade_price,
            spread: r.spread,
            scraped_at: r.scraped_at,
            icon: r.icon,
            one_day_change: r.one_day_change,
        }
    }
}

#[derive(Serialize, sqlx::FromRow)]
pub struct TradeRow {
    ts: DateTime<Utc>,
    proxy_wallet: String,
    name: Option<String>,
    pseudonym: Option<String>,
    condition_id: String,
    title: Option<String>,
    side: String,
    outcome: Option<String>,
    size: f64,
    price: f64,
    transaction_hash: String,
}

const TRADE_SELECT: &str = "SELECT t.ts, t.proxy_wallet, p.name, p.pseudonym, t.condition_id,
        t.title, t.side, t.outcome, t.size::float8 AS size, t.price::float8 AS price,
        t.transaction_hash
    FROM data_trades t
    LEFT JOIN profiles p USING (proxy_wallet)";

// ---------------------------------------------------------------- overview

#[derive(Serialize, sqlx::FromRow)]
struct Totals {
    markets: i64,
    live_markets: i64,
    live_volume: f64,
    live_liquidity: f64,
    wallets: i64,
    trades_24h: i64,
    volume_24h: f64,
}

#[derive(Serialize)]
pub struct Overview {
    totals: Totals,
    top_markets: Vec<Market>,
    recent_trades: Vec<TradeRow>,
}

pub async fn overview(State(pool): State<PgPool>) -> ApiResult<Overview> {
    let totals = sqlx::query_as::<_, Totals>(&format!(
        "SELECT
            (SELECT count(*) FROM markets) AS markets,
            (SELECT count(*) FROM markets WHERE {LIVE}) AS live_markets,
            (SELECT coalesce(sum(volume_num), 0)::float8 FROM markets WHERE {LIVE}) AS live_volume,
            (SELECT coalesce(sum(liquidity_num), 0)::float8 FROM markets WHERE {LIVE}) AS live_liquidity,
            (SELECT count(*) FROM profiles) AS wallets,
            (SELECT count(*) FROM data_trades WHERE ts > now() - interval '24 hours') AS trades_24h,
            (SELECT coalesce(sum(size * price), 0)::float8 FROM data_trades
                WHERE ts > now() - interval '24 hours') AS volume_24h"
    ))
    .fetch_one(&pool)
    .await?;

    let top_markets = sqlx::query_as::<_, MarketRow>(&format!(
        "SELECT {MARKET_COLS} FROM markets WHERE {LIVE}
        ORDER BY volume_num DESC NULLS LAST LIMIT 10"
    ))
    .fetch_all(&pool)
    .await?
    .into_iter()
    .map(Market::from)
    .collect();

    let recent_trades =
        sqlx::query_as::<_, TradeRow>(&format!("{TRADE_SELECT} ORDER BY t.ts DESC LIMIT 15"))
            .fetch_all(&pool)
            .await?;

    Ok(Json(Overview {
        totals,
        top_markets,
        recent_trades,
    }))
}

#[derive(Deserialize)]
pub struct MarketsQuery {
    q: Option<String>,
    status: Option<String>,
    sort: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Serialize)]
pub struct Page<T> {
    total: i64,
    rows: Vec<T>,
}

pub async fn markets(
    State(pool): State<PgPool>,
    Query(q): Query<MarketsQuery>,
) -> ApiResult<Page<Market>> {
    let status = match q.status.as_deref() {
        Some("closed") => format!("NOT ({LIVE})"),
        Some("all") => "true".to_string(),
        _ => LIVE.to_string(),
    };
    let order = match q.sort.as_deref() {
        Some("liquidity") => "liquidity_num DESC NULLS LAST",
        Some("ending") => "end_date ASC NULLS LAST",
        Some("spread") => "spread DESC NULLS LAST",
        Some("movers") => "abs((raw->>'oneDayPriceChange')::float8) DESC NULLS LAST",
        _ => "volume_num DESC NULLS LAST",
    };
    let search = q.q.unwrap_or_default().trim().to_string();

    let where_ = format!(
        "WHERE {status} AND ($1 = '' OR question ILIKE '%' || $1 || '%' OR slug ILIKE '%' || $1 || '%')"
    );

    let total: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM markets {where_}"))
        .bind(&search)
        .fetch_one(&pool)
        .await?;

    let rows = sqlx::query_as::<_, MarketRow>(&format!(
        "SELECT {MARKET_COLS} FROM markets {where_} ORDER BY {order}, id LIMIT $2 OFFSET $3"
    ))
    .bind(&search)
    .bind(page_limit(q.limit))
    .bind(q.offset.unwrap_or(0).max(0))
    .fetch_all(&pool)
    .await?
    .into_iter()
    .map(Market::from)
    .collect();

    Ok(Json(Page { total, rows }))
}

#[derive(Serialize, sqlx::FromRow)]
struct PricePoint {
    token: String,
    ts: DateTime<Utc>,
    price: f64,
}

#[derive(Serialize, sqlx::FromRow)]
struct BookLevel {
    asset_id: String,
    side: String,
    price: f64,
    size: f64,
}

#[derive(Serialize, sqlx::FromRow)]
struct HolderRow {
    token: String,
    proxy_wallet: String,
    name: Option<String>,
    pseudonym: Option<String>,
    amount: f64,
}

#[derive(Serialize, sqlx::FromRow)]
struct MarketInfo {
    description: Option<String>,
    image: Option<String>,
    resolution_source: Option<String>,
    start_date: Option<String>,
    active: Option<bool>,
    accepting_orders: Option<bool>,
    tick_size: Option<f64>,
    min_order_size: Option<f64>,
    one_week_change: Option<f64>,
    one_month_change: Option<f64>,
}

#[derive(Serialize, sqlx::FromRow)]
struct QuoteRow {
    asset_id: String,
    kind: String,
    side: Option<String>,
    value: f64,
    captured_at: DateTime<Utc>,
}

/// Header of a token's newest /books snapshot.
#[derive(Serialize, sqlx::FromRow)]
struct BookMeta {
    asset_id: String,
    ts: DateTime<Utc>,
    tick_size: Option<f64>,
    min_order_size: Option<f64>,
    neg_risk: Option<bool>,
    last_trade_price: Option<f64>,
}

#[derive(Serialize, sqlx::FromRow)]
struct WsTrade {
    asset_id: String,
    ts: DateTime<Utc>,
    price: f64,
    size: Option<f64>,
    side: String,
    fee_rate_bps: Option<f64>,
    transaction_hash: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
struct PriceChange {
    asset_id: String,
    ts: DateTime<Utc>,
    price: f64,
    size: f64,
    side: String,
    best_ask: Option<f64>,
}

#[derive(Serialize, sqlx::FromRow)]
struct TickChange {
    asset_id: String,
    ts: DateTime<Utc>,
    old_tick_size: Option<f64>,
    new_tick_size: f64,
}

#[derive(Serialize)]
pub struct MarketDetail {
    market: Market,
    info: MarketInfo,
    history: Vec<PricePoint>,
    spread_history: Vec<PricePoint>,
    quotes: Vec<QuoteRow>,
    books: Vec<BookMeta>,
    book: Vec<BookLevel>,
    holders: Vec<HolderRow>,
    trades: Vec<TradeRow>,
    ws_trades: Vec<WsTrade>,
    price_changes: Vec<PriceChange>,
    tick_changes: Vec<TickChange>,
}

pub async fn market(State(pool): State<PgPool>, Path(id): Path<String>) -> ApiResult<MarketDetail> {
    let market: Market = sqlx::query_as::<_, MarketRow>(&format!(
        "SELECT {MARKET_COLS} FROM markets WHERE id = $1 OR condition_id = $1 LIMIT 1"
    ))
    .bind(&id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "market not found"))?
    .into();

    let tokens = &market.token_ids;

    let info = sqlx::query_as::<_, MarketInfo>(
        "SELECT raw->>'description' AS description, raw->>'image' AS image,
                raw->>'resolutionSource' AS resolution_source, start_date, active,
                accepting_orders,
                order_price_min_tick_size::float8 AS tick_size,
                order_min_size::float8 AS min_order_size,
                (raw->>'oneWeekPriceChange')::float8 AS one_week_change,
                (raw->>'oneMonthPriceChange')::float8 AS one_month_change
            FROM markets WHERE id = $1",
    )
    .bind(&market.id)
    .fetch_one(&pool)
    .await?;

    let history = sqlx::query_as::<_, PricePoint>(
        "SELECT market AS token, ts, price::float8 AS price
            FROM price_history
        WHERE market = ANY($1) AND ts > now() - interval '30 days'
        ORDER BY ts",
    )
    .bind(tokens)
    .fetch_all(&pool)
    .await?;

    let spread_history = sqlx::query_as::<_, PricePoint>(
        "SELECT asset_id AS token, date_trunc('hour', captured_at) AS ts,
                avg(value)::float8 AS price
            FROM quotes
        WHERE asset_id = ANY($1) AND kind = 'spread'
          AND captured_at > now() - interval '30 days'
        GROUP BY 1, 2
        ORDER BY 2",
    )
    .bind(tokens)
    .fetch_all(&pool)
    .await?;

    let mut quotes = sqlx::query_as::<_, QuoteRow>(
        "SELECT q.asset_id, q.kind, q.side, q.value::float8 AS value, q.captured_at
            FROM unnest($1::text[]) AS t(asset_id)
            CROSS JOIN (VALUES ('midpoint'), ('spread'), ('price')) AS k(kind)
            CROSS JOIN LATERAL (
                SELECT asset_id, kind, side, value, captured_at
                    FROM quotes
                WHERE asset_id = t.asset_id AND kind = k.kind
                ORDER BY captured_at DESC
                LIMIT 2
            ) q",
    )
    .bind(tokens)
    .fetch_all(&pool)
    .await?;
    let mut seen = std::collections::HashSet::new();
    quotes.retain(|q| seen.insert((q.asset_id.clone(), q.kind.clone(), q.side.clone())));

    let books = sqlx::query_as::<_, BookMeta>(
        "SELECT DISTINCT ON (asset_id) asset_id, ts,
                tick_size::float8 AS tick_size, min_order_size::float8 AS min_order_size,
                neg_risk, last_trade_price::float8 AS last_trade_price
            FROM book_snapshots
        WHERE asset_id = ANY($1)
        ORDER BY asset_id, ts DESC",
    )
    .bind(tokens)
    .fetch_all(&pool)
    .await?;

    let book = sqlx::query_as::<_, BookLevel>(
        "WITH latest AS (
            SELECT DISTINCT ON (asset_id) asset_id, ts, hash
                FROM book_snapshots
            WHERE asset_id = ANY($1)
            ORDER BY asset_id, ts DESC
        )
        SELECT l.asset_id, l.side, l.price::float8 AS price, l.size::float8 AS size
            FROM book_levels l
            JOIN latest s USING (asset_id, ts, hash)
        ORDER BY l.asset_id, l.side,
            CASE WHEN l.side = 'bid' THEN -l.price ELSE l.price END",
    )
    .bind(tokens)
    .fetch_all(&pool)
    .await?
    .into_iter()
    .fold(Vec::<BookLevel>::new(), |mut acc, lvl| {
        let same = acc
            .iter()
            .filter(|l| l.asset_id == lvl.asset_id && l.side == lvl.side)
            .count();
        if same < 10 {
            acc.push(lvl);
        }
        acc
    });

    let holders = sqlx::query_as::<_, HolderRow>(
        "WITH latest AS (
            SELECT token, max(captured_at) AS captured_at
                FROM token_holders
            WHERE token = ANY($1)
            GROUP BY token
        ), ranked AS (
            SELECT h.token, h.proxy_wallet, h.amount,
                   row_number() OVER (PARTITION BY h.token ORDER BY h.amount DESC) AS rn
                FROM token_holders h
                JOIN latest USING (token, captured_at)
        )
        SELECT r.token, r.proxy_wallet, p.name, p.pseudonym, r.amount::float8 AS amount
            FROM ranked r
            LEFT JOIN profiles p USING (proxy_wallet)
        WHERE r.rn <= 10
        ORDER BY r.token, r.amount DESC",
    )
    .bind(tokens)
    .fetch_all(&pool)
    .await?;

    let trades = sqlx::query_as::<_, TradeRow>(&format!(
        "{TRADE_SELECT} WHERE t.condition_id = $1 ORDER BY t.ts DESC LIMIT 50"
    ))
    .bind(&market.condition_id)
    .fetch_all(&pool)
    .await?;

    let ws_trades = sqlx::query_as::<_, WsTrade>(
        "SELECT asset_id, ts, price::float8 AS price, size::float8 AS size, side,
                fee_rate_bps::float8 AS fee_rate_bps, transaction_hash
            FROM trades
        WHERE asset_id = ANY($1)
        ORDER BY ts DESC
        LIMIT 50",
    )
    .bind(tokens)
    .fetch_all(&pool)
    .await?;

    let price_changes = sqlx::query_as::<_, PriceChange>(
        "SELECT asset_id, ts, price::float8 AS price, size::float8 AS size, side,
                best_ask::float8 AS best_ask
            FROM price_changes
        WHERE asset_id = ANY($1)
        ORDER BY ts DESC
        LIMIT 50",
    )
    .bind(tokens)
    .fetch_all(&pool)
    .await?;

    let tick_changes = sqlx::query_as::<_, TickChange>(
        "SELECT asset_id, ts, old_tick_size::float8 AS old_tick_size,
                new_tick_size::float8 AS new_tick_size
            FROM tick_size_changes
        WHERE asset_id = ANY($1)
        ORDER BY ts DESC
        LIMIT 20",
    )
    .bind(tokens)
    .fetch_all(&pool)
    .await?;

    Ok(Json(MarketDetail {
        market,
        info,
        history,
        spread_history,
        quotes,
        books,
        book,
        holders,
        trades,
        ws_trades,
        price_changes,
        tick_changes,
    }))
}

#[derive(Deserialize)]
pub struct TradersQuery {
    q: Option<String>,
    sort: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct TraderRow {
    proxy_wallet: String,
    name: Option<String>,
    pseudonym: Option<String>,
    profile_image: Option<String>,
    pnl: Option<f64>,
    value: Option<f64>,
    volume: f64,
    trades: i64,
}

const TRADER_SELECT: &str = "WITH pnl AS (
        SELECT DISTINCT ON (proxy_wallet) proxy_wallet, pnl
            FROM user_pnl ORDER BY proxy_wallet, ts DESC
    ), val AS (
        SELECT DISTINCT ON (proxy_wallet) proxy_wallet, value
            FROM user_values WHERE market = ''
        ORDER BY proxy_wallet, captured_at DESC
    ), vol AS (
        SELECT proxy_wallet, sum(size * price) AS volume, count(*) AS trades
            FROM data_trades GROUP BY proxy_wallet
    )
    SELECT p.proxy_wallet, p.name, p.pseudonym,
           coalesce(p.profile_image_optimized, p.profile_image) AS profile_image,
           pnl.pnl::float8 AS pnl, val.value::float8 AS value,
           coalesce(vol.volume, 0)::float8 AS volume, coalesce(vol.trades, 0) AS trades
        FROM profiles p
        LEFT JOIN pnl USING (proxy_wallet)
        LEFT JOIN val USING (proxy_wallet)
        LEFT JOIN vol USING (proxy_wallet)";

pub async fn traders(
    State(pool): State<PgPool>,
    Query(q): Query<TradersQuery>,
) -> ApiResult<Page<TraderRow>> {
    let order = match q.sort.as_deref() {
        Some("value") => "value DESC NULLS LAST",
        Some("volume") => "volume DESC",
        Some("loss") => "pnl ASC NULLS LAST",
        _ => "pnl DESC NULLS LAST",
    };
    let search = q.q.unwrap_or_default().trim().to_string();
    let where_ = "WHERE $1 = '' OR p.proxy_wallet ILIKE '%' || $1 || '%'
        OR p.name ILIKE '%' || $1 || '%' OR p.pseudonym ILIKE '%' || $1 || '%'";

    let total: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM profiles p {where_}"))
        .bind(&search)
        .fetch_one(&pool)
        .await?;

    let rows = sqlx::query_as::<_, TraderRow>(&format!(
        "{TRADER_SELECT} {where_} ORDER BY {order}, p.proxy_wallet LIMIT $2 OFFSET $3"
    ))
    .bind(&search)
    .bind(page_limit(q.limit))
    .bind(q.offset.unwrap_or(0).max(0))
    .fetch_all(&pool)
    .await?;

    Ok(Json(Page { total, rows }))
}

#[derive(Serialize, sqlx::FromRow)]
struct PnlPoint {
    ts: DateTime<Utc>,
    pnl: f64,
}

#[derive(Serialize, sqlx::FromRow)]
struct ValuePoint {
    ts: DateTime<Utc>,
    value: f64,
}

#[derive(Serialize, sqlx::FromRow)]
struct ProfileInfo {
    bio: Option<String>,
    verified: Option<bool>,
}

#[derive(Serialize, sqlx::FromRow)]
struct PositionRow {
    asset: String,
    condition_id: String,
    has_market: bool,
    question: Option<String>,
    outcome: Option<String>,
    size: f64,
    avg_price: f64,
    cur_price: f64,
    initial_value: f64,
    current_value: f64,
    total_bought: f64,
    cash_pnl: f64,
    percent_pnl: f64,
    realized_pnl: f64,
    redeemable: bool,
    mergeable: bool,
    end_date: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
struct ActivityRow {
    ts: DateTime<Utc>,
    activity_type: String,
    condition_id: String,
    title: Option<String>,
    side: Option<String>,
    outcome: Option<String>,
    size: f64,
    usdc_size: f64,
    price: f64,
    transaction_hash: String,
}

#[derive(Serialize)]
pub struct TraderDetail {
    trader: TraderRow,
    profile: ProfileInfo,
    pnl: Vec<PnlPoint>,
    values: Vec<ValuePoint>,
    positions: Vec<PositionRow>,
    activity: Vec<ActivityRow>,
}

pub async fn trader(
    State(pool): State<PgPool>,
    Path(wallet): Path<String>,
) -> ApiResult<TraderDetail> {
    let trader =
        sqlx::query_as::<_, TraderRow>(&format!("{TRADER_SELECT} WHERE p.proxy_wallet = $1"))
            .bind(&wallet)
            .fetch_optional(&pool)
            .await?
            .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "trader not found"))?;

    let pnl = sqlx::query_as::<_, PnlPoint>(
        "SELECT ts, pnl::float8 AS pnl FROM user_pnl WHERE proxy_wallet = $1 ORDER BY ts",
    )
    .bind(&wallet)
    .fetch_all(&pool)
    .await?;

    let profile =
        sqlx::query_as::<_, ProfileInfo>("SELECT bio, verified FROM profiles WHERE proxy_wallet = $1")
            .bind(&wallet)
            .fetch_one(&pool)
            .await?;

    let values = sqlx::query_as::<_, ValuePoint>(
        "SELECT captured_at AS ts, value::float8 AS value
            FROM user_values
        WHERE proxy_wallet = $1 AND market = ''
        ORDER BY captured_at",
    )
    .bind(&wallet)
    .fetch_all(&pool)
    .await?;

    // Positions are snapshots; the newest capture is the current book.
    let positions = sqlx::query_as::<_, PositionRow>(
        "SELECT ps.asset, ps.condition_id, coalesce(m.found, false) AS has_market,
                -- Positions carry no title; fall back to one a trade or
                -- activity row recorded for the same condition.
                coalesce(
                    m.question,
                    (SELECT title FROM data_trades WHERE condition_id = ps.condition_id LIMIT 1),
                    (SELECT title FROM activity WHERE condition_id = ps.condition_id LIMIT 1)
                ) AS question,
                ps.outcome,
                ps.size::float8 AS size, ps.avg_price::float8 AS avg_price,
                ps.cur_price::float8 AS cur_price, ps.initial_value::float8 AS initial_value,
                ps.current_value::float8 AS current_value, ps.total_bought::float8 AS total_bought,
                ps.cash_pnl::float8 AS cash_pnl, ps.percent_pnl::float8 AS percent_pnl,
                ps.realized_pnl::float8 AS realized_pnl, ps.redeemable, ps.mergeable, ps.end_date
            FROM positions ps
            LEFT JOIN LATERAL (
                SELECT question, true AS found
                    FROM markets WHERE condition_id = ps.condition_id LIMIT 1
            ) m ON true
        WHERE ps.proxy_wallet = $1
          AND ps.captured_at = (SELECT max(captured_at) FROM positions WHERE proxy_wallet = $1)
        ORDER BY ps.current_value DESC",
    )
    .bind(&wallet)
    .fetch_all(&pool)
    .await?;

    let activity = sqlx::query_as::<_, ActivityRow>(
        "SELECT ts, activity_type, condition_id, title, side, outcome,
                size::float8 AS size, usdc_size::float8 AS usdc_size, price::float8 AS price,
                transaction_hash
            FROM activity
        WHERE proxy_wallet = $1
        ORDER BY ts DESC
        LIMIT 50",
    )
    .bind(&wallet)
    .fetch_all(&pool)
    .await?;

    Ok(Json(TraderDetail {
        trader,
        profile,
        pnl,
        values,
        positions,
        activity,
    }))
}

#[derive(Deserialize)]
pub struct TradesQuery {
    side: Option<String>,
    min_usd: Option<f64>,
    market: Option<String>,
    wallet: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

pub async fn trades(
    State(pool): State<PgPool>,
    Query(q): Query<TradesQuery>,
) -> ApiResult<Vec<TradeRow>> {
    let side = q.side.unwrap_or_default().to_uppercase();

    let rows = sqlx::query_as::<_, TradeRow>(&format!(
        "{TRADE_SELECT}
        WHERE ($1 = '' OR t.side = $1) AND t.size * t.price >= $2
          AND ($5 = '' OR t.condition_id = $5) AND ($6 = '' OR t.proxy_wallet = $6)
        ORDER BY t.ts DESC LIMIT $3 OFFSET $4"
    ))
    .bind(&side)
    .bind(q.min_usd.unwrap_or(0.0))
    .bind(page_limit(q.limit))
    .bind(q.offset.unwrap_or(0).max(0))
    .bind(q.market.unwrap_or_default())
    .bind(q.wallet.unwrap_or_default())
    .fetch_all(&pool)
    .await?;

    Ok(Json(rows))
}

#[derive(Deserialize)]
pub struct ActivityQuery {
    #[serde(rename = "type")]
    kind: Option<String>,
    wallet: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ActivityFeedRow {
    ts: DateTime<Utc>,
    proxy_wallet: String,
    name: Option<String>,
    pseudonym: Option<String>,
    activity_type: String,
    condition_id: String,
    title: Option<String>,
    side: Option<String>,
    outcome: Option<String>,
    size: f64,
    usdc_size: f64,
    price: f64,
    transaction_hash: String,
}

pub async fn activity(
    State(pool): State<PgPool>,
    Query(q): Query<ActivityQuery>,
) -> ApiResult<Vec<ActivityFeedRow>> {
    let rows = sqlx::query_as::<_, ActivityFeedRow>(
        "SELECT a.ts, a.proxy_wallet, p.name, p.pseudonym, a.activity_type, a.condition_id,
                a.title, a.side, a.outcome, a.size::float8 AS size,
                a.usdc_size::float8 AS usdc_size, a.price::float8 AS price, a.transaction_hash
            FROM activity a
            LEFT JOIN profiles p USING (proxy_wallet)
        WHERE ($1 = '' OR a.activity_type = $1) AND ($2 = '' OR a.proxy_wallet = $2)
        ORDER BY a.ts DESC LIMIT $3 OFFSET $4",
    )
    .bind(q.kind.unwrap_or_default().to_uppercase())
    .bind(q.wallet.unwrap_or_default())
    .bind(page_limit(q.limit))
    .bind(q.offset.unwrap_or(0).max(0))
    .fetch_all(&pool)
    .await?;

    Ok(Json(rows))
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn page_limit_defaults_to_50_and_clamps_to_1_200() {
        assert_eq!(page_limit(None), 50);
        assert_eq!(page_limit(Some(0)), 1);
        assert_eq!(page_limit(Some(-5)), 1);
        assert_eq!(page_limit(Some(120)), 120);
        assert_eq!(page_limit(Some(10_000)), 200);
    }

    #[test]
    fn json_list_reads_arrays_inside_strings() {
        assert_eq!(json_list(Some(r#"["Yes", "No"]"#)), vec!["Yes", "No"]);
        assert!(json_list(Some("not json")).is_empty());
        assert!(json_list(Some("")).is_empty());
        assert!(json_list(None).is_empty());
    }

    fn row() -> MarketRow {
        MarketRow {
            id: "1".into(),
            question: Some("Q?".into()),
            slug: None,
            condition_id: "0xc".into(),
            category: None,
            closed: None,
            end_date: None,
            volume: None,
            liquidity: None,
            best_bid: None,
            best_ask: None,
            last_trade_price: None,
            spread: None,
            outcomes: Some(r#"["Yes","No"]"#.into()),
            outcome_prices: Some(r#"["0.25","0.75"]"#.into()),
            clob_token_ids: Some(r#"["t1","t2"]"#.into()),
            scraped_at: Utc::now(),
            icon: None,
            one_day_change: Some(0.05),
        }
    }

    #[test]
    fn market_from_row_unpacks_stringified_lists() {
        let m = Market::from(row());
        assert_eq!(m.outcomes, vec!["Yes", "No"]);
        assert_eq!(m.outcome_prices, vec![0.25, 0.75]);
        assert_eq!(m.token_ids, vec!["t1", "t2"]);
        assert!(!m.closed, "a missing closed flag reads as open");
        assert_eq!(m.one_day_change, Some(0.05));
    }

    #[test]
    fn market_from_row_skips_unparseable_prices() {
        let mut r = row();
        r.outcome_prices = Some(r#"["0.4","oops"]"#.into());
        r.closed = Some(true);
        let m = Market::from(r);
        assert_eq!(m.outcome_prices, vec![0.4]);
        assert!(m.closed);
    }
}
