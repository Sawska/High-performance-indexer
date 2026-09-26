use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use futures_util::stream::{self, StreamExt};
use sqlx::PgPool;

use crate::db::rest_writer::{self, Quote};
use crate::stats;
use crate::venues::polymarket::clob::ClobApi;
use crate::venues::polymarket::data::DataApi;
use crate::venues::polymarket::gamma::GammaApi;
use crate::venues::polymarket::types::{
    ActivityQuery, Holder, ListMarkets, PositionsQuery, PriceHistoryQuery, Profile, Trade,
    TradesQuery,
};
use crate::venues::polymarket::user_pnl::UserPnlApi;

type Res<T> = Result<T, String>;

macro_rules! warn {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        eprintln!("{msg}");
        stats::error(msg);
    }};
}

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

pub struct Config {
    pub interval: Duration,
    pub market_page_limit: i32,
    pub max_markets: usize,
    pub token_chunk: usize,
    pub concurrency: usize,
    pub max_markets_deep: usize,
    pub max_trade_pages: usize,
    pub wallets_per_cycle: usize,
    pub holders_limit: i32,
    pub positions_limit: i32,
    pub activity_limit: i32,
    pub history_interval: String,
    pub pnl_interval: String,
    pub pnl_fidelity: String,
    pub include_closed: bool,
}

fn env_or<T: FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(300),
            market_page_limit: 500,
            max_markets: 0,
            token_chunk: 50,
            concurrency: 8,
            max_markets_deep: 500,
            max_trade_pages: 5,
            wallets_per_cycle: 200,
            holders_limit: 100,
            positions_limit: 100,
            activity_limit: 100,
            history_interval: "1d".to_string(),
            pnl_interval: "all".to_string(),
            pnl_fidelity: "1d".to_string(),
            include_closed: false,
        }
    }
}

impl Config {
    pub fn from_env() -> Self {
        let d = Self::default();
        Self {
            interval: Duration::from_secs(env_or("SCRAPE_INTERVAL_SECS", d.interval.as_secs())),
            market_page_limit: env_or("SCRAPE_MARKET_PAGE_LIMIT", d.market_page_limit),
            max_markets: env_or("SCRAPE_MAX_MARKETS", d.max_markets),
            token_chunk: env_or("SCRAPE_TOKEN_CHUNK", d.token_chunk),
            concurrency: env_or("SCRAPE_CONCURRENCY", d.concurrency),
            max_markets_deep: env_or("SCRAPE_MAX_MARKETS_DEEP", d.max_markets_deep),
            max_trade_pages: env_or("SCRAPE_MAX_TRADE_PAGES", d.max_trade_pages),
            wallets_per_cycle: env_or("SCRAPE_WALLETS_PER_CYCLE", d.wallets_per_cycle),
            holders_limit: env_or("SCRAPE_HOLDERS_LIMIT", d.holders_limit),
            positions_limit: env_or("SCRAPE_POSITIONS_LIMIT", d.positions_limit),
            activity_limit: env_or("SCRAPE_ACTIVITY_LIMIT", d.activity_limit),
            history_interval: env_or("SCRAPE_HISTORY_INTERVAL", d.history_interval),
            pnl_interval: env_or("SCRAPE_PNL_INTERVAL", d.pnl_interval),
            pnl_fidelity: env_or("SCRAPE_PNL_FIDELITY", d.pnl_fidelity),
            include_closed: env_or("SCRAPE_INCLUDE_CLOSED", d.include_closed),
        }
    }
}

struct Apis {
    gamma: GammaApi,
    clob: ClobApi,
    data: DataApi,
    pnl: UserPnlApi,
}

impl Apis {
    fn new() -> Self {
        Self {
            gamma: GammaApi::new(),
            clob: ClobApi::new(),
            data: DataApi::new(),
            pnl: UserPnlApi::new(),
        }
    }
}

struct MarketRef {
    condition_id: String,
    token_ids: Vec<String>,
}

fn parse_token_ids(raw: Option<&str>) -> Vec<String> {
    raw.and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
        .unwrap_or_default()
}

fn is_live(m: &ListMarkets) -> bool {
    !m.closed.unwrap_or(false) && !m.archived.unwrap_or(false)
}

fn profile_from_holder(h: &Holder) -> Profile {
    Profile {
        proxy_wallet: h.proxy_wallet.clone(),
        name: h.name.clone(),
        pseudonym: h.pseudonym.clone(),
        bio: h.bio.clone(),
        display_username_public: h.display_username_public,
        profile_image: h.profile_image.clone(),
        profile_image_optimized: h.profile_image_optimized.clone(),
        verified: h.verified,
    }
}

fn profile_from_trade(t: &Trade) -> Profile {
    Profile {
        proxy_wallet: t.proxy_wallet.clone(),
        name: t.name.clone(),
        pseudonym: t.pseudonym.clone(),
        bio: t.bio.clone(),
        display_username_public: None,
        profile_image: t.profile_image.clone(),
        profile_image_optimized: t.profile_image_optimized.clone(),
        verified: None,
    }
}

pub async fn run(pool: PgPool, cfg: Config) {
    let apis = Apis::new();
    let mut cycle_no: u64 = 0;

    loop {
        let started = Instant::now();
        stats::cycle_start(cycle_no);

        if let Err(e) = cycle(&pool, &cfg, &apis, cycle_no).await {
            warn!("scrape: cycle {cycle_no} failed: {e}");
        }

        let elapsed = started.elapsed();
        println!("scrape: cycle {cycle_no} done in {:?}", elapsed);
        stats::cycle_end(elapsed.as_millis() as u64);
        cycle_no = cycle_no.wrapping_add(1);

        if let Some(rest) = cfg.interval.checked_sub(elapsed) {
            tokio::time::sleep(rest).await;
        }
    }
}

async fn cycle(pool: &PgPool, cfg: &Config, apis: &Apis, cycle_no: u64) -> Res<()> {
    let scraped_at = Utc::now();

    let markets = scrape_markets(pool, cfg, apis, scraped_at).await?;

    let tokens: Vec<String> = markets
        .iter()
        .flat_map(|m| m.token_ids.iter().cloned())
        .collect();

    println!(
        "scrape: {} markets, {} tokens",
        markets.len(),
        tokens.len()
    );
    stats::set(|s| {
        s.live_markets = markets.len() as u64;
        s.tokens = tokens.len() as u64;
    });

    scrape_books(pool, cfg, apis, &tokens, scraped_at).await;
    scrape_quotes(pool, cfg, apis, &tokens).await;
    scrape_history(pool, cfg, apis, &tokens).await;

    let deep: &[MarketRef] = if cfg.max_markets_deep == 0 {
        &markets
    } else {
        &markets[..markets.len().min(cfg.max_markets_deep)]
    };

    let mut wallets: HashSet<String> = HashSet::new();
    let mut profiles: HashMap<String, Profile> = HashMap::new();

    scrape_trades(pool, cfg, apis, deep, scraped_at, &mut wallets, &mut profiles).await;
    scrape_holders(pool, cfg, apis, deep, &mut wallets, &mut profiles).await;

    let discovered: Vec<Profile> = profiles.into_values().collect();
    if let Err(e) = rest_writer::upsert_profiles(pool, &discovered, scraped_at).await {
        warn!("scrape: profiles: {e}");
    }

    let mut wallets: Vec<String> = wallets.into_iter().collect();
    wallets.sort();
    let slice = rotating_slice(&wallets, cfg.wallets_per_cycle, cycle_no);
    println!(
        "scrape: {} wallets discovered, {} this cycle",
        wallets.len(),
        slice.len()
    );
    stats::set(|s| {
        s.wallets_discovered = wallets.len() as u64;
        s.wallets_this_cycle = slice.len() as u64;
    });

    scrape_wallets(pool, cfg, apis, slice, scraped_at).await;

    Ok(())
}

fn rotating_slice<T>(items: &[T], size: usize, cycle_no: u64) -> &[T] {
    if size == 0 || items.len() <= size {
        return items;
    }
    let windows = items.len().div_ceil(size);
    let start = (cycle_no as usize % windows) * size;
    &items[start..items.len().min(start + size)]
}

async fn scrape_markets(
    pool: &PgPool,
    cfg: &Config,
    apis: &Apis,
    scraped_at: DateTime<Utc>,
) -> Res<Vec<MarketRef>> {
    let mut refs: Vec<MarketRef> = Vec::new();
    let mut cursor: Option<String> = None;
    let mut total = 0usize;
    stats::stage_start("markets", 0);

    loop {
        let page = apis
            .gamma
            .list_markets(cfg.market_page_limit, cursor.as_deref())
            .await
            .map_err(err)?;

        if page.markets.is_empty() {
            break;
        }

        rest_writer::upsert_markets(pool, &page.markets, scraped_at)
            .await
            .map_err(err)?;

        total += page.markets.len();
        stats::tick();
        stats::set(|s| s.markets = total as u64);

        for m in &page.markets {
            if !cfg.include_closed && !is_live(m) {
                continue;
            }
            let token_ids = parse_token_ids(m.clob_token_ids.as_deref());
            if token_ids.is_empty() {
                continue;
            }
            refs.push(MarketRef {
                condition_id: m.condition_id.clone(),
                token_ids,
            });
        }

        if cfg.max_markets != 0 && total >= cfg.max_markets {
            break;
        }

        match page.next_cursor {
            Some(next) if Some(&next) != cursor.as_ref() => cursor = Some(next),
            _ => break,
        }
    }

    Ok(refs)
}

async fn scrape_books(
    pool: &PgPool,
    cfg: &Config,
    apis: &Apis,
    tokens: &[String],
    scraped_at: DateTime<Utc>,
) {
    let chunks: Vec<&[String]> = tokens.chunks(cfg.token_chunk.max(1)).collect();

    stats::stage_start("books", chunks.len());
    stream::iter(chunks)
        .map(|chunk| async move {
            let ids: Vec<&str> = chunk.iter().map(String::as_str).collect();

            let books = match apis.clob.get_books(&ids).await {
                Ok(b) => b,
                Err(e) => {
                    warn!("scrape: books: {e}");
                    return;
                }
            };

            if let Err(e) = rest_writer::insert_books(pool, &books, scraped_at).await {
                warn!("scrape: insert books: {e}");
            }
        })
        .buffer_unordered(cfg.concurrency.max(1))
        .inspect(|_| stats::tick())
        .collect::<()>()
        .await;
}

async fn scrape_quotes(pool: &PgPool, cfg: &Config, apis: &Apis, tokens: &[String]) {
    let chunks: Vec<&[String]> = tokens.chunks(cfg.token_chunk.max(1)).collect();

    stats::stage_start("quotes", chunks.len());
    stream::iter(chunks)
        .map(|chunk| async move {
            let captured_at = Utc::now();
            let ids: Vec<&str> = chunk.iter().map(String::as_str).collect();
            let mut quotes: Vec<Quote> = Vec::new();

            match apis.clob.get_midpoints(&ids).await {
                Ok(map) => quotes.extend(scalar_quotes(map, "midpoint")),
                Err(e) => warn!("scrape: midpoints: {e}"),
            }

            match apis.clob.get_spreads(&ids).await {
                Ok(map) => quotes.extend(scalar_quotes(map, "spread")),
                Err(e) => warn!("scrape: spreads: {e}"),
            }

            let params: Vec<(&str, &str)> = ids
                .iter()
                .flat_map(|t| [(*t, "BUY"), (*t, "SELL")])
                .collect();

            match apis.clob.get_prices(&params).await {
                Ok(by_token) => {
                    for (asset_id, by_side) in by_token {
                        for (side, raw) in by_side {
                            let Ok(value) = raw.parse::<f64>() else {
                                continue;
                            };
                            quotes.push(Quote {
                                asset_id: asset_id.clone(),
                                kind: "price".to_string(),
                                side: Some(side),
                                value,
                            });
                        }
                    }
                }
                Err(e) => warn!("scrape: prices: {e}"),
            }

            if let Err(e) = rest_writer::insert_quotes(pool, &quotes, captured_at).await {
                warn!("scrape: insert quotes: {e}");
            }
        })
        .buffer_unordered(cfg.concurrency.max(1))
        .inspect(|_| stats::tick())
        .collect::<()>()
        .await;
}

fn scalar_quotes(map: HashMap<String, String>, kind: &str) -> Vec<Quote> {
    map.into_iter()
        .filter_map(|(asset_id, raw)| {
            Some(Quote {
                asset_id,
                kind: kind.to_string(),
                side: None,
                value: raw.parse::<f64>().ok()?,
            })
        })
        .collect()
}

async fn scrape_history(pool: &PgPool, cfg: &Config, apis: &Apis, tokens: &[String]) {
    stats::stage_start("history", tokens.len());
    stream::iter(tokens)
        .map(|token| async move {
            let mut query = PriceHistoryQuery::new(token);
            query.interval = Some(cfg.history_interval.as_str());

            let history = match apis.clob.get_history(query).await {
                Ok(h) => h.history,
                Err(e) => {
                    warn!("scrape: history {token}: {e}");
                    return;
                }
            };

            if let Err(e) = rest_writer::insert_price_history(pool, token, &history).await {
                warn!("scrape: insert history {token}: {e}");
            }
        })
        .buffer_unordered(cfg.concurrency.max(1))
        .inspect(|_| stats::tick())
        .collect::<()>()
        .await;
}

async fn scrape_trades(
    pool: &PgPool,
    cfg: &Config,
    apis: &Apis,
    markets: &[MarketRef],
    scraped_at: DateTime<Utc>,
    wallets: &mut HashSet<String>,
    profiles: &mut HashMap<String, Profile>,
) {
    stats::stage_start("trades", markets.len());
    let found: Vec<Vec<Trade>> = stream::iter(markets)
        .map(|m| async move {
            let mut all: Vec<Trade> = Vec::new();
            let mut cursor: Option<String> = None;

            for _ in 0..cfg.max_trade_pages.max(1) {
                let query = TradesQuery {
                    condition_id: Some(m.condition_id.as_str()),
                    cursor: cursor.as_deref(),
                    ..Default::default()
                };

                let page = match apis.data.get_trades(query).await {
                    Ok(p) => p,
                    Err(e) => {
                        warn!("scrape: trades {}: {e}", m.condition_id);
                        break;
                    }
                };

                all.extend(page.data);

                match page.pagination.next_cursor {
                    Some(next) if page.pagination.has_more => cursor = Some(next),
                    _ => break,
                }
            }

            if let Err(e) = rest_writer::insert_data_trades(pool, &all, scraped_at).await {
                warn!("scrape: insert trades {}: {e}", m.condition_id);
            }

            all
        })
        .buffer_unordered(cfg.concurrency.max(1))
        .inspect(|_| stats::tick())
        .collect()
        .await;

    for trade in found.iter().flatten() {
        wallets.insert(trade.proxy_wallet.clone());
        profiles
            .entry(trade.proxy_wallet.clone())
            .or_insert_with(|| profile_from_trade(trade));
    }
}

async fn scrape_holders(
    pool: &PgPool,
    cfg: &Config,
    apis: &Apis,
    markets: &[MarketRef],
    wallets: &mut HashSet<String>,
    profiles: &mut HashMap<String, Profile>,
) {
    stats::stage_start("holders", markets.len());
    let found: Vec<Vec<Holder>> = stream::iter(markets)
        .map(|m| async move {
            let captured_at = Utc::now();

            let holders = match apis
                .data
                .get_holders(&m.condition_id, cfg.holders_limit)
                .await
            {
                Ok(h) => h,
                Err(e) => {
                    warn!("scrape: holders {}: {e}", m.condition_id);
                    return Vec::new();
                }
            };

            if let Err(e) = rest_writer::insert_holders(pool, &holders, captured_at).await {
                warn!("scrape: insert holders {}: {e}", m.condition_id);
            }

            holders.into_iter().flat_map(|mh| mh.holders).collect()
        })
        .buffer_unordered(cfg.concurrency.max(1))
        .inspect(|_| stats::tick())
        .collect()
        .await;

    for holder in found.iter().flatten() {
        wallets.insert(holder.proxy_wallet.clone());
        // A holder carries the fuller profile, so it overwrites a trade's.
        profiles.insert(holder.proxy_wallet.clone(), profile_from_holder(holder));
    }
}

async fn scrape_wallets(
    pool: &PgPool,
    cfg: &Config,
    apis: &Apis,
    wallets: &[String],
    scraped_at: DateTime<Utc>,
) {
    stats::stage_start("wallets", wallets.len());
    stream::iter(wallets)
        .map(|wallet| async move {
            let captured_at = Utc::now();

            let positions = PositionsQuery {
                limit: cfg.positions_limit,
                ..PositionsQuery::new(wallet)
            };

            match apis.data.get_positions(positions).await {
                Ok(p) => {
                    if let Err(e) = rest_writer::insert_positions(pool, &p, captured_at).await {
                        warn!("scrape: insert positions {wallet}: {e}");
                    }
                }
                Err(e) => warn!("scrape: positions {wallet}: {e}"),
            }

            let activity = ActivityQuery {
                limit: cfg.activity_limit,
                ..ActivityQuery::new(wallet)
            };

            match apis.data.get_activity(activity).await {
                Ok(a) => {
                    if let Err(e) = rest_writer::insert_activity(pool, &a, scraped_at).await {
                        warn!("scrape: insert activity {wallet}: {e}");
                    }
                }
                Err(e) => warn!("scrape: activity {wallet}: {e}"),
            }

            match apis.data.get_value(wallet, None).await {
                Ok(v) => {
                    if let Err(e) = rest_writer::insert_user_values(pool, &v, None, captured_at).await
                    {
                        warn!("scrape: insert value {wallet}: {e}");
                    }
                }
                Err(e) => warn!("scrape: value {wallet}: {e}"),
            }

            match apis
                .pnl
                .get_pnl(wallet, &cfg.pnl_interval, &cfg.pnl_fidelity)
                .await
            {
                Ok(points) => {
                    if let Err(e) = rest_writer::insert_user_pnl(pool, wallet, &points).await {
                        warn!("scrape: insert pnl {wallet}: {e}");
                    }
                }
                Err(e) => warn!("scrape: pnl {wallet}: {e}"),
            }
        })
        .buffer_unordered(cfg.concurrency.max(1))
        .inspect(|_| stats::tick())
        .collect::<()>()
        .await;
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn rotating_slice_covers_everything_across_cycles() {
        let items: Vec<u32> = (0..10).collect();
        assert_eq!(rotating_slice(&items, 0, 7), &items[..], "0 = no cap");
        assert_eq!(rotating_slice(&items, 10, 3), &items[..]);
        assert_eq!(rotating_slice(&items, 4, 0), &[0, 1, 2, 3]);
        assert_eq!(rotating_slice(&items, 4, 1), &[4, 5, 6, 7]);
        assert_eq!(rotating_slice(&items, 4, 2), &[8, 9], "last window is short");
        assert_eq!(rotating_slice(&items, 4, 3), &[0, 1, 2, 3], "wraps around");

        let mut seen: Vec<u32> = (0..3).flat_map(|c| rotating_slice(&items, 4, c).to_vec()).collect();
        seen.sort();
        assert_eq!(seen, items);
    }

    #[test]
    fn parse_token_ids_reads_the_stringified_array() {
        assert_eq!(parse_token_ids(Some(r#"["1","2"]"#)), vec!["1", "2"]);
        assert!(parse_token_ids(Some("[]")).is_empty());
        assert!(parse_token_ids(Some("garbage")).is_empty());
        assert!(parse_token_ids(None).is_empty());
    }

    fn market(json: &str) -> ListMarkets {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn is_live_excludes_closed_and_archived() {
        assert!(is_live(&market(r#"{"id":"1","conditionId":"c"}"#)));
        assert!(is_live(&market(r#"{"id":"1","conditionId":"c","closed":false,"archived":false}"#)));
        assert!(!is_live(&market(r#"{"id":"1","conditionId":"c","closed":true}"#)));
        assert!(!is_live(&market(r#"{"id":"1","conditionId":"c","archived":true}"#)));
    }

    #[test]
    fn scalar_quotes_drop_unparseable_values() {
        let map = HashMap::from([
            ("a".to_string(), "0.5".to_string()),
            ("b".to_string(), "".to_string()),
        ]);
        let quotes = scalar_quotes(map, "midpoint");
        assert_eq!(quotes.len(), 1);
        assert_eq!(quotes[0].asset_id, "a");
        assert_eq!(quotes[0].kind, "midpoint");
        assert_eq!(quotes[0].side, None);
        assert_eq!(quotes[0].value, 0.5);
    }

    #[test]
    fn env_overrides_config_and_bad_values_fall_back() {
        // Keys no other test reads, so setting them cannot race.
        unsafe {
            std::env::set_var("SCRAPE_TEST_ONLY_NUM", "42");
            std::env::set_var("SCRAPE_TEST_ONLY_BAD", "forty-two");
        }
        assert_eq!(env_or("SCRAPE_TEST_ONLY_NUM", 7usize), 42);
        assert_eq!(env_or("SCRAPE_TEST_ONLY_BAD", 7usize), 7);
        assert_eq!(env_or("SCRAPE_TEST_ONLY_UNSET", 7usize), 7);
    }
}
