use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::venues::polymarket::types::MarketEvent;

const BATCH_SIZE: usize = 1000;
const FLUSH_MS: u64 = 500;
const CHANNEL_CAP: usize = 10_000;

pub struct TradeRow {
    pub asset_id: String,
    pub market: String,
    pub ts: DateTime<Utc>,
    pub price: f64,
    pub size: Option<f64>,
    pub side: String,
    pub transaction_hash: Option<String>,
}

pub struct PriceChangeRow {
    pub asset_id: String,
    pub market: String,
    pub ts: DateTime<Utc>,
    pub price: f64,
    pub size: f64,
    pub side: String,
    pub best_ask: Option<f64>,
    pub hash: Option<String>,
}

pub struct TickSizeRow {
    pub asset_id: String,
    pub market: String,
    pub ts: DateTime<Utc>,
    pub old_tick_size: Option<f64>,
    pub new_tick_size: f64,
}

#[derive(Default)]
struct Batch {
    trades: Vec<TradeRow>,
    price_changes: Vec<PriceChangeRow>,
    tick_sizes: Vec<TickSizeRow>,
}

impl Batch {
    fn len(&self) -> usize {
        self.trades.len() + self.price_changes.len() + self.tick_sizes.len()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub fn spawn(pool: PgPool) -> mpsc::Sender<MarketEvent> {
    let (tx, mut rx) = mpsc::channel::<MarketEvent>(CHANNEL_CAP);

    tokio::spawn(async move {
        let mut batch = Batch::default();
        let mut tick = tokio::time::interval(std::time::Duration::from_millis(FLUSH_MS));

        loop {
            tokio::select! {
                maybe_ev = rx.recv() => match maybe_ev {
                    Some(ev) => {
                        push_event(&mut batch, ev);
                        if batch.len() >= BATCH_SIZE {
                            flush(&pool, &mut batch).await;
                        }
                    }
                    None => {
                        flush(&pool, &mut batch).await;
                        break;
                    }
                },
                _ = tick.tick() => flush(&pool, &mut batch).await,
            }
        }
    });

    tx
}

fn parse_ts(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::from_timestamp_millis(raw.parse::<i64>().ok()?)
}

fn parse_num(raw: &str) -> Option<f64> {
    raw.parse::<f64>().ok()
}

fn push_event(batch: &mut Batch, ev: MarketEvent) {
    match ev {
        MarketEvent::LastTradePrice(e) => {
            let (Some(ts), Some(price)) = (
                e.timestamp.as_deref().and_then(parse_ts),
                parse_num(&e.price),
            ) else {
                eprintln!("trade: unparseable ts/price for asset {}", e.asset_id);
                return;
            };
            batch.trades.push(TradeRow {
                asset_id: e.asset_id,
                market: e.market,
                ts,
                price,
                size: e.size.as_deref().and_then(parse_num),
                side: e.side,
                transaction_hash: e.transaction_hash,
            });
        }

        MarketEvent::PriceChange(e) => {
            let Some(ts) = parse_ts(&e.timestamp) else {
                eprintln!("price_change: unparseable ts for market {}", e.market);
                return;
            };
            for c in e.price_changes {
                let (Some(price), Some(size)) = (parse_num(&c.price), parse_num(&c.size)) else {
                    continue;
                };
                batch.price_changes.push(PriceChangeRow {
                    asset_id: c.asset_id,
                    market: e.market.clone(),
                    ts,
                    price,
                    size,
                    side: c.side,
                    best_ask: parse_num(&c.best_ask),
                    hash: Some(c.hash),
                });
            }
        }

        MarketEvent::TickSizeChange(e) => {
            let (Some(ts), Some(new_tick_size)) = (
                e.timestamp.as_deref().and_then(parse_ts),
                parse_num(&e.new_tick_size),
            ) else {
                eprintln!("tick_size: unparseable ts/size for asset {}", e.asset_id);
                return;
            };
            batch.tick_sizes.push(TickSizeRow {
                asset_id: e.asset_id,
                market: e.market,
                ts,
                old_tick_size: e.old_tick_size.as_deref().and_then(parse_num),
                new_tick_size,
            });
        }

        MarketEvent::Book(_) => {}
        MarketEvent::Unknow => {}
    }
}

async fn flush(pool: &PgPool, batch: &mut Batch) {
    if batch.is_empty() {
        return;
    }
    flush_trades(pool, &mut batch.trades).await;
    flush_price_changes(pool, &mut batch.price_changes).await;
    flush_tick_sizes(pool, &mut batch.tick_sizes).await;
}

async fn flush_trades(pool: &PgPool, buf: &mut Vec<TradeRow>) {
    if buf.is_empty() {
        return;
    }

    let asset_ids: Vec<String> = buf.iter().map(|r| r.asset_id.clone()).collect();
    let markets: Vec<String> = buf.iter().map(|r| r.market.clone()).collect();
    let tss: Vec<DateTime<Utc>> = buf.iter().map(|r| r.ts).collect();
    let prices: Vec<f64> = buf.iter().map(|r| r.price).collect();
    let sizes: Vec<Option<f64>> = buf.iter().map(|r| r.size).collect();
    let sides: Vec<String> = buf.iter().map(|r| r.side.clone()).collect();
    let hashes: Vec<Option<String>> = buf.iter().map(|r| r.transaction_hash.clone()).collect();

    let res = sqlx::query(
        "INSERT INTO trades
             (asset_id, market, ts, price, size, side, transaction_hash)
         SELECT * FROM UNNEST(
             $1::text[], $2::text[], $3::timestamptz[], $4::float8[],
             $5::float8[], $6::text[], $7::text[])",
    )
    .bind(&asset_ids)
    .bind(&markets)
    .bind(&tss)
    .bind(&prices)
    .bind(&sizes)
    .bind(&sides)
    .bind(&hashes)
    .execute(pool)
    .await;

    if let Err(e) = res {
        eprintln!("trades flush failed ({} rows): {e}", buf.len());
    }
    buf.clear();
}

async fn flush_price_changes(pool: &PgPool, buf: &mut Vec<PriceChangeRow>) {
    if buf.is_empty() {
        return;
    }

    let asset_ids: Vec<String> = buf.iter().map(|r| r.asset_id.clone()).collect();
    let markets: Vec<String> = buf.iter().map(|r| r.market.clone()).collect();
    let tss: Vec<DateTime<Utc>> = buf.iter().map(|r| r.ts).collect();
    let prices: Vec<f64> = buf.iter().map(|r| r.price).collect();
    let sizes: Vec<f64> = buf.iter().map(|r| r.size).collect();
    let sides: Vec<String> = buf.iter().map(|r| r.side.clone()).collect();
    let best_asks: Vec<Option<f64>> = buf.iter().map(|r| r.best_ask).collect();
    let hashes: Vec<Option<String>> = buf.iter().map(|r| r.hash.clone()).collect();

    let res = sqlx::query(
        "INSERT INTO price_changes
             (asset_id, market, ts, price, size, side, best_ask, hash)
         SELECT * FROM UNNEST(
             $1::text[], $2::text[], $3::timestamptz[], $4::float8[],
             $5::float8[], $6::text[], $7::float8[], $8::text[])",
    )
    .bind(&asset_ids)
    .bind(&markets)
    .bind(&tss)
    .bind(&prices)
    .bind(&sizes)
    .bind(&sides)
    .bind(&best_asks)
    .bind(&hashes)
    .execute(pool)
    .await;

    if let Err(e) = res {
        eprintln!("price_changes flush failed ({} rows): {e}", buf.len());
    }
    buf.clear();
}

async fn flush_tick_sizes(pool: &PgPool, buf: &mut Vec<TickSizeRow>) {
    if buf.is_empty() {
        return;
    }

    let asset_ids: Vec<String> = buf.iter().map(|r| r.asset_id.clone()).collect();
    let markets: Vec<String> = buf.iter().map(|r| r.market.clone()).collect();
    let tss: Vec<DateTime<Utc>> = buf.iter().map(|r| r.ts).collect();
    let olds: Vec<Option<f64>> = buf.iter().map(|r| r.old_tick_size).collect();
    let news: Vec<f64> = buf.iter().map(|r| r.new_tick_size).collect();

    let res = sqlx::query(
        "INSERT INTO tick_size_changes
             (asset_id, market, ts, old_tick_size, new_tick_size)
         SELECT * FROM UNNEST(
             $1::text[], $2::text[], $3::timestamptz[], $4::float8[], $5::float8[])",
    )
    .bind(&asset_ids)
    .bind(&markets)
    .bind(&tss)
    .bind(&olds)
    .bind(&news)
    .execute(pool)
    .await;

    if let Err(e) = res {
        eprintln!("tick_size_changes flush failed ({} rows): {e}", buf.len());
    }
    buf.clear();
}
