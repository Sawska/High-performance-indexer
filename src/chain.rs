//! The Polygon log indexer.
//!
//! Shape of a cycle: ask the node for the head, stay `confirmations` blocks
//! behind it, pull a batch of block ranges concurrently, resolve the timestamps
//! of the blocks those logs landed in, write everything, then move the cursor.
//!
//! Staying behind the head is what makes reorgs a non-event: by the time a
//! block is indexed it is buried. The stored block hash is still checked each
//! cycle so that a reorg deeper than the confirmation depth is caught and
//! rewound rather than silently leaving a forked range in the database.

use std::collections::HashSet;
use std::str::FromStr;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use futures_util::stream::{self, StreamExt};
use serde::Serialize;
use sqlx::PgPool;

use crate::db::chain_writer::{self, BlockTimes};
use crate::venues::polymarket::onchain::consts;
use crate::venues::polymarket::onchain::events::{
    self, CollateralTransfer, Condition, CtfPosition, Decoded, OrderFill, Redemption,
    TokenTransfer,
};
use crate::venues::polymarket::onchain::rpc::{Log, Rpc};

const CURSOR: &str = "polygon";

fn env_or<T: FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

pub struct Config {
    pub rpc_url: String,
    /// Absolute block to begin at. 0 means "head minus `initial_lookback`",
    /// which keeps a first run on a free RPC from trying to walk all of 2020.
    pub start_block: u64,
    pub initial_lookback: u64,
    pub confirmations: u64,
    pub log_range: u64,
    pub concurrency: usize,
    pub poll_interval: Duration,
    pub request_timeout: Duration,
    pub max_retries: u32,
    pub backoff: Duration,
    pub block_batch: usize,
    pub fetch_block_times: bool,
    pub reorg_rewind: u64,
    pub feed_exchange: bool,
    pub feed_ctf: bool,
    pub feed_tokens: bool,
    pub feed_collateral: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rpc_url: consts::POLYGON_RPC.to_string(),
            start_block: 0,
            initial_lookback: 100_000, // ~2.5 days of Polygon blocks
            confirmations: 64,
            log_range: 1_000,
            concurrency: 2,
            poll_interval: Duration::from_secs(5),
            request_timeout: Duration::from_secs(30),
            max_retries: 5,
            backoff: Duration::from_millis(500),
            block_batch: 100,
            fetch_block_times: true,
            reorg_rewind: 128,
            feed_exchange: true,
            feed_ctf: true,
            feed_tokens: true,
            feed_collateral: true,
        }
    }
}

impl Config {
    pub fn from_env() -> Self {
        let d = Self::default();
        Self {
            rpc_url: std::env::var("CHAIN_RPC_URL").unwrap_or(d.rpc_url),
            start_block: env_or("CHAIN_START_BLOCK", d.start_block),
            initial_lookback: env_or("CHAIN_INITIAL_LOOKBACK", d.initial_lookback),
            confirmations: env_or("CHAIN_CONFIRMATIONS", d.confirmations),
            log_range: env_or("CHAIN_LOG_RANGE", d.log_range).max(1),
            concurrency: env_or("CHAIN_CONCURRENCY", d.concurrency).max(1),
            poll_interval: Duration::from_secs(env_or(
                "CHAIN_POLL_SECS",
                d.poll_interval.as_secs(),
            )),
            request_timeout: Duration::from_secs(env_or(
                "CHAIN_REQUEST_TIMEOUT_SECS",
                d.request_timeout.as_secs(),
            )),
            max_retries: env_or("CHAIN_MAX_RETRIES", d.max_retries),
            backoff: Duration::from_millis(env_or("CHAIN_BACKOFF_MS", d.backoff.as_millis() as u64)),
            block_batch: env_or("CHAIN_BLOCK_BATCH", d.block_batch).max(1),
            fetch_block_times: env_or("CHAIN_FETCH_BLOCK_TIMES", d.fetch_block_times),
            reorg_rewind: env_or("CHAIN_REORG_REWIND", d.reorg_rewind),
            feed_exchange: env_or("CHAIN_FEED_EXCHANGE", d.feed_exchange),
            feed_ctf: env_or("CHAIN_FEED_CTF", d.feed_ctf),
            feed_tokens: env_or("CHAIN_FEED_TOKENS", d.feed_tokens),
            feed_collateral: env_or("CHAIN_FEED_COLLATERAL", d.feed_collateral),
        }
    }

    pub fn enabled() -> bool {
        env_or("CHAIN_ENABLED", false)
    }

    /// Topics to ask the ConditionalTokens contract for, given which of the two
    /// feeds that share it are switched on.
    fn ctf_topics(&self) -> Vec<String> {
        let mut topics = Vec::new();
        if self.feed_ctf {
            topics.extend([
                consts::TOPIC_POSITION_SPLIT.clone(),
                consts::TOPIC_POSITIONS_MERGE.clone(),
                consts::TOPIC_PAYOUT_REDEMPTION.clone(),
                consts::TOPIC_CONDITION_PREPARATION.clone(),
                consts::TOPIC_CONDITION_RESOLUTION.clone(),
            ]);
        }
        if self.feed_tokens {
            topics.extend([
                consts::TOPIC_TRANSFER_SINGLE.clone(),
                consts::TOPIC_TRANSFER_BATCH.clone(),
            ]);
        }
        topics
    }
}

// ── observable state ────────────────────────────────────────────────────────

#[derive(Clone, Default, Serialize)]
pub struct Status {
    pub enabled: bool,
    pub head: Option<i64>,
    pub cursor: Option<i64>,
    /// Blocks between the cursor and the head; how far behind the indexer is.
    pub lag: Option<i64>,
    pub last_range: Option<(i64, i64)>,
    pub ranges_done: u64,
    pub rows_written: u64,
    pub reorgs: u64,
    pub errors: u64,
    pub last_error: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

static STATUS: LazyLock<Mutex<Status>> = LazyLock::new(|| Mutex::new(Status::default()));

fn with_status<R>(f: impl FnOnce(&mut Status) -> R) -> R {
    let mut s = STATUS.lock().unwrap_or_else(|e| e.into_inner());
    let r = f(&mut s);
    s.updated_at = Some(Utc::now());
    r
}

pub fn status() -> Status {
    STATUS.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

fn warn(msg: String) {
    eprintln!("chain: {msg}");
    with_status(|s| {
        s.errors += 1;
        s.last_error = Some(msg);
    });
}

// ── per-range results ───────────────────────────────────────────────────────

#[derive(Default)]
struct RangeData {
    fills: Vec<OrderFill>,
    positions: Vec<CtfPosition>,
    redemptions: Vec<Redemption>,
    conditions: Vec<Condition>,
    tokens: Vec<TokenTransfer>,
    collateral: Vec<CollateralTransfer>,
    blocks: HashSet<i64>,
}

impl RangeData {
    fn absorb(&mut self, other: RangeData) {
        self.fills.extend(other.fills);
        self.positions.extend(other.positions);
        self.redemptions.extend(other.redemptions);
        self.conditions.extend(other.conditions);
        self.tokens.extend(other.tokens);
        self.collateral.extend(other.collateral);
        self.blocks.extend(other.blocks);
    }

    fn push(&mut self, decoded: Decoded, block: i64) {
        self.blocks.insert(block);
        match decoded {
            Decoded::Fill(f) => self.fills.push(f),
            Decoded::Position(p) => self.positions.push(p),
            Decoded::Redemption(r) => self.redemptions.push(r),
            Decoded::Condition(c) => self.conditions.push(c),
            Decoded::Tokens(rows) => self.tokens.extend(rows),
            Decoded::Collateral(t) => self.collateral.push(t),
        }
    }

    fn rows(&self) -> usize {
        self.fills.len()
            + self.positions.len()
            + self.redemptions.len()
            + self.conditions.len()
            + self.tokens.len()
            + self.collateral.len()
    }
}

/// A log that a reorg already removed is never indexed; `eth_getLogs` should
/// not return these for a canonical range, but the flag is honoured anyway.
fn live(log: &Log) -> bool {
    !log.removed
}

async fn fetch_range(rpc: &Rpc, cfg: &Config, from: u64, to: u64) -> Result<RangeData, String> {
    let mut data = RangeData::default();

    if cfg.feed_exchange {
        let addresses: Vec<String> = consts::EXCHANGES
            .iter()
            .map(|e| e.address.to_string())
            .collect();
        let logs = rpc
            .get_logs(from, to, &addresses, &consts::exchange_topics())
            .await
            .map_err(|e| format!("exchange logs {from}-{to}: {e}"))?;

        for log in logs.iter().filter(|l| live(l)) {
            if let (Some(d), Some(b)) = (events::decode_exchange(log), log.block()) {
                data.push(d, b as i64);
            }
        }
    }

    let ctf_topics = cfg.ctf_topics();
    if !ctf_topics.is_empty() {
        let addresses = vec![consts::CONDITIONAL_TOKENS.to_string()];
        let logs = rpc
            .get_logs(from, to, &addresses, &ctf_topics)
            .await
            .map_err(|e| format!("ctf logs {from}-{to}: {e}"))?;

        for log in logs.iter().filter(|l| live(l)) {
            if let (Some(d), Some(b)) = (events::decode_ctf(log), log.block()) {
                data.push(d, b as i64);
            }
        }
    }

    if cfg.feed_collateral {
        let addresses: Vec<String> = consts::COLLATERALS
            .iter()
            .map(|c| c.address.to_string())
            .collect();
        let topic = vec![consts::TOPIC_ERC20_TRANSFER.clone()];
        let watched: Vec<String> = consts::WATCHED_COUNTERPARTIES
            .iter()
            .map(|a| format!("0x{:0>64}", a.trim_start_matches("0x")))
            .collect();

        // `topics` is an AND across positions, so "from is ours OR to is ours"
        // cannot be expressed in one filter and needs a query per side.
        for position in [1_usize, 2] {
            let logs = rpc
                .get_logs_with_topic(from, to, &addresses, &topic, position, &watched)
                .await
                .map_err(|e| format!("collateral logs {from}-{to} (topic {position}): {e}"))?;

            for log in logs.iter().filter(|l| live(l)) {
                if let (Some(d), Some(b)) = (events::decode_collateral(log), log.block()) {
                    data.push(d, b as i64);
                }
            }
        }
    }

    Ok(data)
}

/// Fill in timestamps for the blocks this batch touched, fetching only the ones
/// not already cached.
async fn resolve_times(
    pool: &PgPool,
    rpc: &Rpc,
    cfg: &Config,
    blocks: &HashSet<i64>,
) -> BlockTimes {
    if blocks.is_empty() {
        return BlockTimes::new();
    }

    let mut wanted: Vec<i64> = blocks.iter().copied().collect();
    wanted.sort_unstable();

    let cached = chain_writer::block_times(pool, &wanted).await.unwrap_or_default();
    if !cfg.fetch_block_times {
        return cached;
    }

    let missing: Vec<u64> = wanted
        .iter()
        .filter(|n| !cached.contains_key(n))
        .map(|n| *n as u64)
        .collect();

    if missing.is_empty() {
        return cached;
    }

    let fetched = rpc.blocks(&missing, cfg.block_batch).await;
    if fetched.len() < missing.len() {
        warn(format!(
            "block times: got {} of {} headers; those rows keep a null block_time",
            fetched.len(),
            missing.len()
        ));
    }

    if let Err(e) = chain_writer::insert_blocks(pool, &fetched).await {
        warn(format!("caching block headers: {e}"));
    }

    let mut times = cached;
    for b in fetched {
        if let Some(ts) = DateTime::from_timestamp(b.timestamp, 0) {
            times.insert(b.number as i64, ts);
        }
    }
    times
}

async fn write(pool: &PgPool, data: &RangeData, times: &BlockTimes) -> u64 {
    let mut written = 0;

    macro_rules! store {
        ($call:expr, $what:literal) => {
            match $call.await {
                Ok(n) => written += n,
                Err(e) => warn(format!("writing {}: {e}", $what)),
            }
        };
    }

    store!(chain_writer::insert_fills(pool, &data.fills, times), "order fills");
    store!(chain_writer::insert_positions(pool, &data.positions, times), "splits/merges");
    store!(chain_writer::insert_redemptions(pool, &data.redemptions, times), "redemptions");
    store!(chain_writer::upsert_conditions(pool, &data.conditions, times), "conditions");
    store!(chain_writer::insert_token_transfers(pool, &data.tokens, times), "token transfers");
    store!(
        chain_writer::insert_collateral_transfers(pool, &data.collateral, times),
        "collateral transfers"
    );

    written
}

/// Detect a reorg deeper than the confirmation depth by re-reading the block the
/// cursor points at. Returns the block to resume from.
async fn check_reorg(pool: &PgPool, rpc: &Rpc, cfg: &Config, last: i64, stored: &str) -> i64 {
    let current = match rpc.block(last as u64).await {
        Ok(Some(b)) => b.hash.to_lowercase(),
        // Can't tell: leave the cursor alone rather than rewinding blindly.
        Ok(None) => return last,
        Err(e) => {
            warn(format!("reorg check at {last}: {e}"));
            return last;
        }
    };

    if current == stored.to_lowercase() {
        return last;
    }

    let target = last.saturating_sub(cfg.reorg_rewind as i64).max(0);
    warn(format!(
        "reorg at block {last}: stored {stored} is now {current}; rewinding to {target}"
    ));

    if let Err(e) = chain_writer::rewind(pool, target).await {
        warn(format!("rewind to {target}: {e}"));
        return last;
    }
    if let Err(e) = chain_writer::save_cursor(pool, CURSOR, target, None).await {
        warn(format!("saving rewound cursor: {e}"));
    }

    with_status(|s| s.reorgs += 1);
    target
}

pub async fn run(pool: PgPool, cfg: Config) {
    let rpc = Rpc::new(
        cfg.rpc_url.clone(),
        cfg.request_timeout,
        cfg.max_retries,
        cfg.backoff,
    );

    with_status(|s| s.enabled = true);
    println!(
        "chain: indexing polygon via {} (range {}, {} in flight, {} confirmations)",
        cfg.rpc_url, cfg.log_range, cfg.concurrency, cfg.confirmations
    );

    // Where to resume. A stored cursor always wins over configuration, so
    // changing CHAIN_START_BLOCK cannot silently punch a hole in the history.
    let mut next: Option<i64> = match chain_writer::load_cursor(&pool, CURSOR).await {
        Ok(Some((last, hash))) => {
            let resume = match hash {
                Some(h) => check_reorg(&pool, &rpc, &cfg, last, &h).await,
                None => last,
            };
            println!("chain: resuming from block {}", resume + 1);
            Some(resume + 1)
        }
        Ok(None) => None,
        Err(e) => {
            warn(format!("reading cursor: {e}"));
            None
        }
    };

    loop {
        let head = match rpc.block_number().await {
            Ok(h) => h,
            Err(e) => {
                warn(format!("head: {e}"));
                tokio::time::sleep(cfg.poll_interval).await;
                continue;
            }
        };
        with_status(|s| s.head = Some(head as i64));

        let safe = head.saturating_sub(cfg.confirmations);

        // First ever run: anchor the cursor relative to the current head.
        let from = match next {
            Some(n) => n as u64,
            None => {
                let start = if cfg.start_block > 0 {
                    cfg.start_block
                } else {
                    safe.saturating_sub(cfg.initial_lookback)
                };
                println!("chain: starting at block {start}");
                next = Some(start as i64);
                start
            }
        };

        if from > safe {
            with_status(|s| {
                s.cursor = Some(from as i64 - 1);
                s.lag = Some(head as i64 - (from as i64 - 1));
            });
            tokio::time::sleep(cfg.poll_interval).await;
            continue;
        }

        // A batch of consecutive ranges, fetched concurrently and committed as
        // one unit so the cursor only ever moves over fully written blocks.
        let mut ranges = Vec::new();
        let mut cursor = from;
        while cursor <= safe && ranges.len() < cfg.concurrency {
            let end = (cursor + cfg.log_range - 1).min(safe);
            ranges.push((cursor, end));
            cursor = end + 1;
        }
        let Some(&(_, batch_end)) = ranges.last() else {
            tokio::time::sleep(cfg.poll_interval).await;
            continue;
        };

        let results: Vec<Result<RangeData, String>> = stream::iter(ranges.clone())
            .map(|(a, b)| fetch_range(&rpc, &cfg, a, b))
            .buffered(cfg.concurrency)
            .collect()
            .await;

        let mut data = RangeData::default();
        let mut failed = false;
        for r in results {
            match r {
                Ok(d) => data.absorb(d),
                Err(e) => {
                    warn(e);
                    failed = true;
                }
            }
        }

        // Any gap means the batch is incomplete; retry the whole batch rather
        // than advancing the cursor past blocks that were never read.
        if failed {
            tokio::time::sleep(cfg.backoff * 4).await;
            continue;
        }

        let times = resolve_times(&pool, &rpc, &cfg, &data.blocks).await;
        let rows = data.rows();
        let written = write(&pool, &data, &times).await;

        // The tip's hash is what the next cycle's reorg check compares against.
        let tip_hash = match rpc.block(batch_end).await {
            Ok(Some(b)) => Some(b.hash.to_lowercase()),
            _ => None,
        };

        if let Err(e) =
            chain_writer::save_cursor(&pool, CURSOR, batch_end as i64, tip_hash.as_deref()).await
        {
            // Without a saved cursor the batch would be replayed; the inserts
            // are idempotent, so stopping here is safe.
            warn(format!("saving cursor at {batch_end}: {e}"));
            tokio::time::sleep(cfg.backoff * 4).await;
            continue;
        }

        next = Some(batch_end as i64 + 1);
        with_status(|s| {
            s.cursor = Some(batch_end as i64);
            s.lag = Some(head as i64 - batch_end as i64);
            s.last_range = Some((from as i64, batch_end as i64));
            s.ranges_done += ranges.len() as u64;
            s.rows_written += written;
        });

        if rows > 0 {
            println!(
                "chain: {from}-{batch_end} decoded {rows} rows ({} fills, {} transfers), {written} new",
                data.fills.len(),
                data.tokens.len() + data.collateral.len()
            );
        }

        // Caught up: wait for new blocks. Still behind: keep going, but yield
        // briefly so a free endpoint is not hammered flat out.
        if batch_end >= safe {
            tokio::time::sleep(cfg.poll_interval).await;
        } else {
            tokio::time::sleep(cfg.backoff).await;
        }
    }
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn range_batching_never_passes_the_safe_head() {
        let cfg = Config { log_range: 100, concurrency: 3, ..Config::default() };
        let (from, safe) = (1_000_u64, 1_250_u64);

        let mut ranges = Vec::new();
        let mut cursor = from;
        while cursor <= safe && ranges.len() < cfg.concurrency {
            let end = (cursor + cfg.log_range - 1).min(safe);
            ranges.push((cursor, end));
            cursor = end + 1;
        }

        assert_eq!(ranges, vec![(1000, 1099), (1100, 1199), (1200, 1250)]);
        assert_eq!(ranges.last().unwrap().1, safe, "the last range stops at safe");
        // Ranges are contiguous, so no block is skipped between them.
        for pair in ranges.windows(2) {
            assert_eq!(pair[0].1 + 1, pair[1].0);
        }
    }

    #[test]
    fn ctf_topics_follow_the_feed_switches() {
        let both = Config { feed_ctf: true, feed_tokens: true, ..Config::default() };
        assert_eq!(both.ctf_topics().len(), 7);

        let lifecycle = Config { feed_ctf: true, feed_tokens: false, ..Config::default() };
        assert_eq!(lifecycle.ctf_topics().len(), 5);
        assert!(!lifecycle.ctf_topics().contains(&consts::TOPIC_TRANSFER_BATCH.clone()));

        let transfers = Config { feed_ctf: false, feed_tokens: true, ..Config::default() };
        assert_eq!(transfers.ctf_topics().len(), 2);

        let off = Config { feed_ctf: false, feed_tokens: false, ..Config::default() };
        assert!(off.ctf_topics().is_empty(), "no topics means the query is skipped");
    }

    #[test]
    fn range_data_absorbs_and_counts() {
        let mut a = RangeData::default();
        a.push(
            Decoded::Tokens(vec![TokenTransfer {
                transaction_hash: "0x1".into(),
                log_index: 0,
                item_index: 0,
                block_number: 5,
                contract: "0xc".into(),
                operator: None,
                from_address: "0xf".into(),
                to_address: "0xt".into(),
                token_id: "1".into(),
                amount: 1.0,
            }]),
            5,
        );

        let mut b = RangeData::default();
        b.push(
            Decoded::Condition(Condition {
                condition_id: "0xc1".into(),
                oracle: None,
                question_id: None,
                outcome_slot_count: Some(2),
                resolved: false,
                payout_numerators: None,
                transaction_hash: "0x2".into(),
                block_number: 6,
            }),
            6,
        );

        a.absorb(b);
        assert_eq!(a.rows(), 2);
        assert_eq!(a.blocks, HashSet::from([5, 6]), "both blocks need a timestamp");
    }

    #[test]
    fn removed_logs_are_skipped() {
        let mut log: Log = serde_json::from_str(
            r#"{"address":"0xa","topics":[],"data":"0x","blockNumber":"0x1",
                "transactionHash":"0xt","logIndex":"0x0"}"#,
        )
        .unwrap();
        assert!(live(&log));
        log.removed = true;
        assert!(!live(&log));
    }

    #[test]
    fn watched_counterparties_pad_to_topic_width() {
        let padded = format!(
            "0x{:0>64}",
            consts::CTF_EXCHANGE_V2.trim_start_matches("0x")
        );
        assert_eq!(padded.len(), 66, "a topic is 32 bytes of hex plus 0x");
        assert!(padded.ends_with(consts::CTF_EXCHANGE_V2.trim_start_matches("0x")));
        assert!(padded.starts_with("0x000000000000000000000000"));
    }
}
