use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;

use crate::venues::polymarket::types::{
    Activity, History, ListMarkets, MarketHolders, OrderBook, Position, Profile, Trade, UserPnlPoint,
    UserValue,
};

fn ts_secs(secs: i64) -> Option<DateTime<Utc>> {
    DateTime::from_timestamp(secs, 0)
}

fn ts_millis(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::from_timestamp_millis(raw.parse::<i64>().ok()?)
}

fn parse_num(raw: &str) -> Option<f64> {
    raw.parse::<f64>().ok()
}

pub async fn upsert_markets(
    pool: &PgPool,
    markets: &[ListMarkets],
    scraped_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    if markets.is_empty() {
        return Ok(());
    }

    let mut seen = std::collections::HashSet::new();
    let uniq: Vec<&ListMarkets> = markets
        .iter()
        .rev()
        .filter(|m| seen.insert(m.id.as_str()))
        .collect();

    let ids: Vec<String> = uniq.iter().map(|m| m.id.clone()).collect();
    let questions: Vec<Option<String>> = uniq.iter().map(|m| m.question.clone()).collect();
    let condition_ids: Vec<String> = uniq.iter().map(|m| m.condition_id.clone()).collect();
    let slugs: Vec<Option<String>> = uniq.iter().map(|m| m.slug.clone()).collect();
    let categories: Vec<Option<String>> = uniq.iter().map(|m| m.category.clone()).collect();
    let actives: Vec<Option<bool>> = uniq.iter().map(|m| m.active).collect();
    let closeds: Vec<Option<bool>> = uniq.iter().map(|m| m.closed).collect();
    let archiveds: Vec<Option<bool>> = uniq.iter().map(|m| m.archived).collect();
    let accepting: Vec<Option<bool>> = uniq.iter().map(|m| m.accepting_orders).collect();
    let order_book: Vec<Option<bool>> = uniq.iter().map(|m| m.enable_order_book).collect();
    let starts: Vec<Option<String>> = uniq.iter().map(|m| m.start_date.clone()).collect();
    let ends: Vec<Option<String>> = uniq.iter().map(|m| m.end_date.clone()).collect();
    let outcomes: Vec<Option<String>> = uniq.iter().map(|m| m.outcomes.clone()).collect();
    let outcome_prices: Vec<Option<String>> =
        uniq.iter().map(|m| m.outcome_prices.clone()).collect();
    let clob_token_ids: Vec<Option<String>> =
        uniq.iter().map(|m| m.clob_token_ids.clone()).collect();
    let volumes: Vec<Option<f64>> = uniq.iter().map(|m| m.volume_num).collect();
    let liquidities: Vec<Option<f64>> = uniq.iter().map(|m| m.liquidity_num).collect();
    let best_bids: Vec<Option<f64>> = uniq.iter().map(|m| m.best_bid).collect();
    let best_asks: Vec<Option<f64>> = uniq.iter().map(|m| m.best_ask).collect();
    let last_trades: Vec<Option<f64>> = uniq.iter().map(|m| m.last_trade_price).collect();
    let spreads: Vec<Option<f64>> = uniq.iter().map(|m| m.spread).collect();
    let tick_sizes: Vec<Option<f64>> =
        uniq.iter().map(|m| m.order_price_min_tick_size).collect();
    let min_sizes: Vec<Option<f64>> = uniq.iter().map(|m| m.order_min_size).collect();
    let raws: Vec<Value> = uniq
        .iter()
        .map(|m| serde_json::to_value(m).unwrap_or(Value::Null))
        .collect();
    let scraped: Vec<DateTime<Utc>> = vec![scraped_at; uniq.len()];

    sqlx::query(
        "INSERT INTO markets
             (id, question, condition_id, slug, category, active, closed, archived,
              accepting_orders, enable_order_book, start_date, end_date, outcomes,
              outcome_prices, clob_token_ids, volume_num, liquidity_num, best_bid,
              best_ask, last_trade_price, spread, order_price_min_tick_size,
              order_min_size, raw, scraped_at)
         SELECT * FROM UNNEST(
             $1::text[], $2::text[], $3::text[], $4::text[], $5::text[], $6::bool[],
             $7::bool[], $8::bool[], $9::bool[], $10::bool[], $11::text[], $12::text[],
             $13::text[], $14::text[], $15::text[], $16::float8[], $17::float8[],
             $18::float8[], $19::float8[], $20::float8[], $21::float8[], $22::float8[],
             $23::float8[], $24::jsonb[], $25::timestamptz[])
         ON CONFLICT (id) DO UPDATE SET
             question                  = EXCLUDED.question,
             condition_id              = EXCLUDED.condition_id,
             slug                      = EXCLUDED.slug,
             category                  = EXCLUDED.category,
             active                    = EXCLUDED.active,
             closed                    = EXCLUDED.closed,
             archived                  = EXCLUDED.archived,
             accepting_orders          = EXCLUDED.accepting_orders,
             enable_order_book         = EXCLUDED.enable_order_book,
             start_date                = EXCLUDED.start_date,
             end_date                  = EXCLUDED.end_date,
             outcomes                  = EXCLUDED.outcomes,
             outcome_prices            = EXCLUDED.outcome_prices,
             clob_token_ids            = EXCLUDED.clob_token_ids,
             volume_num                = EXCLUDED.volume_num,
             liquidity_num             = EXCLUDED.liquidity_num,
             best_bid                  = EXCLUDED.best_bid,
             best_ask                  = EXCLUDED.best_ask,
             last_trade_price          = EXCLUDED.last_trade_price,
             spread                    = EXCLUDED.spread,
             order_price_min_tick_size = EXCLUDED.order_price_min_tick_size,
             order_min_size            = EXCLUDED.order_min_size,
             raw                       = EXCLUDED.raw,
             scraped_at                = EXCLUDED.scraped_at",
    )
    .bind(&ids).bind(&questions).bind(&condition_ids).bind(&slugs).bind(&categories)
    .bind(&actives).bind(&closeds).bind(&archiveds).bind(&accepting).bind(&order_book)
    .bind(&starts).bind(&ends).bind(&outcomes).bind(&outcome_prices).bind(&clob_token_ids)
    .bind(&volumes).bind(&liquidities).bind(&best_bids).bind(&best_asks).bind(&last_trades)
    .bind(&spreads).bind(&tick_sizes).bind(&min_sizes).bind(&raws).bind(&scraped)
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn upsert_profiles(
    pool: &PgPool,
    profiles: &[Profile],
    scraped_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    if profiles.is_empty() {
        return Ok(());
    }

    let mut seen = std::collections::HashSet::new();
    let uniq: Vec<&Profile> = profiles
        .iter()
        .rev()
        .filter(|p| seen.insert(p.proxy_wallet.as_str()))
        .collect();

    let wallets: Vec<String> = uniq.iter().map(|p| p.proxy_wallet.clone()).collect();
    let names: Vec<Option<String>> = uniq.iter().map(|p| p.name.clone()).collect();
    let pseudonyms: Vec<Option<String>> = uniq.iter().map(|p| p.pseudonym.clone()).collect();
    let bios: Vec<Option<String>> = uniq.iter().map(|p| p.bio.clone()).collect();
    let images: Vec<Option<String>> = uniq.iter().map(|p| p.profile_image.clone()).collect();
    let images_opt: Vec<Option<String>> =
        uniq.iter().map(|p| p.profile_image_optimized.clone()).collect();
    let verified: Vec<Option<bool>> = uniq.iter().map(|p| p.verified).collect();
    let public: Vec<Option<bool>> = uniq.iter().map(|p| p.display_username_public).collect();
    let scraped: Vec<DateTime<Utc>> = vec![scraped_at; uniq.len()];

    sqlx::query(
        "INSERT INTO profiles
             (proxy_wallet, name, pseudonym, bio, profile_image,
              profile_image_optimized, verified, display_username_public, scraped_at)
         SELECT * FROM UNNEST(
             $1::text[], $2::text[], $3::text[], $4::text[], $5::text[],
             $6::text[], $7::bool[], $8::bool[], $9::timestamptz[])
         ON CONFLICT (proxy_wallet) DO UPDATE SET
             name                    = EXCLUDED.name,
             pseudonym               = EXCLUDED.pseudonym,
             bio                     = EXCLUDED.bio,
             profile_image           = EXCLUDED.profile_image,
             profile_image_optimized = EXCLUDED.profile_image_optimized,
             verified                = EXCLUDED.verified,
             display_username_public = EXCLUDED.display_username_public,
             scraped_at              = EXCLUDED.scraped_at",
    )
    .bind(&wallets).bind(&names).bind(&pseudonyms).bind(&bios).bind(&images)
    .bind(&images_opt).bind(&verified).bind(&public).bind(&scraped)
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn insert_books(
    pool: &PgPool,
    books: &[OrderBook],
    scraped_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    if books.is_empty() {
        return Ok(());
    }

    let usable: Vec<(&OrderBook, DateTime<Utc>)> = books
        .iter()
        .filter_map(|b| ts_millis(&b.timestamp).map(|t| (b, t)))
        .collect();
    if usable.is_empty() {
        return Ok(());
    }

    let asset_ids: Vec<String> = usable.iter().map(|(b, _)| b.asset_id.clone()).collect();
    let markets: Vec<String> = usable.iter().map(|(b, _)| b.market.clone()).collect();
    let tss: Vec<DateTime<Utc>> = usable.iter().map(|(_, t)| *t).collect();
    let hashes: Vec<String> = usable.iter().map(|(b, _)| b.hash.clone()).collect();
    let min_sizes: Vec<Option<f64>> = usable
        .iter()
        .map(|(b, _)| b.min_order_size.as_deref().and_then(parse_num))
        .collect();
    let tick_sizes: Vec<Option<f64>> = usable
        .iter()
        .map(|(b, _)| b.tick_size.as_deref().and_then(parse_num))
        .collect();
    let neg_risks: Vec<Option<bool>> = usable.iter().map(|(b, _)| b.neg_risk).collect();
    let last_trades: Vec<Option<f64>> = usable
        .iter()
        .map(|(b, _)| b.last_trade_price.as_deref().and_then(parse_num))
        .collect();
    let scraped: Vec<DateTime<Utc>> = vec![scraped_at; usable.len()];

    let mut l_asset: Vec<String> = Vec::new();
    let mut l_ts: Vec<DateTime<Utc>> = Vec::new();
    let mut l_hash: Vec<String> = Vec::new();
    let mut l_side: Vec<String> = Vec::new();
    let mut l_idx: Vec<i32> = Vec::new();
    let mut l_price: Vec<f64> = Vec::new();
    let mut l_size: Vec<f64> = Vec::new();

    for (b, t) in &usable {
        for (side, levels) in [("bid", &b.bids), ("ask", &b.asks)] {
            for (i, lvl) in levels.iter().enumerate() {
                let (Some(price), Some(size)) = (parse_num(&lvl.price), parse_num(&lvl.size))
                else {
                    continue;
                };
                l_asset.push(b.asset_id.clone());
                l_ts.push(*t);
                l_hash.push(b.hash.clone());
                l_side.push(side.to_string());
                l_idx.push(i as i32);
                l_price.push(price);
                l_size.push(size);
            }
        }
    }

    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO book_snapshots
             (asset_id, market, ts, hash, min_order_size, tick_size, neg_risk,
              last_trade_price, scraped_at)
         SELECT * FROM UNNEST(
             $1::text[], $2::text[], $3::timestamptz[], $4::text[], $5::float8[],
             $6::float8[], $7::bool[], $8::float8[], $9::timestamptz[])
         ON CONFLICT DO NOTHING",
    )
    .bind(&asset_ids).bind(&markets).bind(&tss).bind(&hashes).bind(&min_sizes)
    .bind(&tick_sizes).bind(&neg_risks).bind(&last_trades).bind(&scraped)
    .execute(&mut *tx)
    .await?;

    if !l_asset.is_empty() {
        sqlx::query(
            "INSERT INTO book_levels
                 (asset_id, ts, hash, side, level_idx, price, size)
             SELECT * FROM UNNEST(
                 $1::text[], $2::timestamptz[], $3::text[], $4::text[],
                 $5::int4[], $6::float8[], $7::float8[])
             ON CONFLICT DO NOTHING",
        )
        .bind(&l_asset).bind(&l_ts).bind(&l_hash).bind(&l_side)
        .bind(&l_idx).bind(&l_price).bind(&l_size)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await
}

pub struct Quote {
    pub asset_id: String,
    pub kind: String,
    pub side: Option<String>,
    pub value: f64,
}

pub async fn insert_quotes(
    pool: &PgPool,
    quotes: &[Quote],
    captured_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    if quotes.is_empty() {
        return Ok(());
    }

    let asset_ids: Vec<String> = quotes.iter().map(|q| q.asset_id.clone()).collect();
    let kinds: Vec<String> = quotes.iter().map(|q| q.kind.clone()).collect();
    let sides: Vec<Option<String>> = quotes.iter().map(|q| q.side.clone()).collect();
    let values: Vec<f64> = quotes.iter().map(|q| q.value).collect();
    let ats: Vec<DateTime<Utc>> = vec![captured_at; quotes.len()];

    sqlx::query(
        "INSERT INTO quotes (asset_id, kind, side, value, captured_at)
         SELECT * FROM UNNEST(
             $1::text[], $2::text[], $3::text[], $4::float8[], $5::timestamptz[])",
    )
    .bind(&asset_ids).bind(&kinds).bind(&sides).bind(&values).bind(&ats)
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn insert_price_history(
    pool: &PgPool,
    market: &str,
    history: &[History],
) -> Result<(), sqlx::Error> {
    if history.is_empty() {
        return Ok(());
    }

    let usable: Vec<(DateTime<Utc>, f64)> = history
        .iter()
        .filter_map(|h| ts_secs(h.t as i64).map(|t| (t, h.p)))
        .collect();

    let markets: Vec<String> = vec![market.to_string(); usable.len()];
    let tss: Vec<DateTime<Utc>> = usable.iter().map(|(t, _)| *t).collect();
    let prices: Vec<f64> = usable.iter().map(|(_, p)| *p).collect();

    sqlx::query(
        "INSERT INTO price_history (market, ts, price)
         SELECT * FROM UNNEST($1::text[], $2::timestamptz[], $3::float8[])
         ON CONFLICT (market, ts) DO NOTHING",
    )
    .bind(&markets).bind(&tss).bind(&prices)
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn insert_data_trades(
    pool: &PgPool,
    trades: &[Trade],
    scraped_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    if trades.is_empty() {
        return Ok(());
    }

    let usable: Vec<(&Trade, DateTime<Utc>)> = trades
        .iter()
        .filter_map(|t| ts_secs(t.timestamp).map(|ts| (t, ts)))
        .collect();
    if usable.is_empty() {
        return Ok(());
    }

    let hashes: Vec<String> = usable.iter().map(|(t, _)| t.transaction_hash.clone()).collect();
    let wallets: Vec<String> = usable.iter().map(|(t, _)| t.proxy_wallet.clone()).collect();
    let condition_ids: Vec<String> = usable.iter().map(|(t, _)| t.condition_id.clone()).collect();
    let token_ids: Vec<String> = usable.iter().map(|(t, _)| t.token_id.clone()).collect();
    let sides: Vec<String> = usable.iter().map(|(t, _)| t.side.clone()).collect();
    let sizes: Vec<f64> = usable.iter().map(|(t, _)| t.size).collect();
    let prices: Vec<f64> = usable.iter().map(|(t, _)| t.price).collect();
    let tss: Vec<DateTime<Utc>> = usable.iter().map(|(_, ts)| *ts).collect();
    let outcomes: Vec<Option<String>> = usable.iter().map(|(t, _)| t.outcome.clone()).collect();
    let outcome_idx: Vec<Option<i64>> = usable.iter().map(|(t, _)| t.outcome_index).collect();
    let titles: Vec<Option<String>> = usable.iter().map(|(t, _)| t.title.clone()).collect();
    let slugs: Vec<Option<String>> = usable.iter().map(|(t, _)| t.slug.clone()).collect();
    let event_slugs: Vec<Option<String>> = usable.iter().map(|(t, _)| t.event_slug.clone()).collect();
    let scraped: Vec<DateTime<Utc>> = vec![scraped_at; usable.len()];

    sqlx::query(
        "INSERT INTO data_trades
             (transaction_hash, proxy_wallet, condition_id, token_id, side, size,
              price, ts, outcome, outcome_index, title, slug, event_slug, scraped_at)
         SELECT * FROM UNNEST(
             $1::text[], $2::text[], $3::text[], $4::text[], $5::text[], $6::float8[],
             $7::float8[], $8::timestamptz[], $9::text[], $10::int8[], $11::text[],
             $12::text[], $13::text[], $14::timestamptz[])
         ON CONFLICT DO NOTHING",
    )
    .bind(&hashes).bind(&wallets).bind(&condition_ids).bind(&token_ids).bind(&sides)
    .bind(&sizes).bind(&prices).bind(&tss).bind(&outcomes).bind(&outcome_idx)
    .bind(&titles).bind(&slugs).bind(&event_slugs).bind(&scraped)
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn insert_positions(
    pool: &PgPool,
    positions: &[Position],
    captured_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    if positions.is_empty() {
        return Ok(());
    }

    let wallets: Vec<String> = positions.iter().map(|p| p.proxy_wallet.clone()).collect();
    let assets: Vec<String> = positions.iter().map(|p| p.asset.clone()).collect();
    let ats: Vec<DateTime<Utc>> = vec![captured_at; positions.len()];
    let condition_ids: Vec<String> = positions.iter().map(|p| p.condition_id.clone()).collect();
    let sizes: Vec<f64> = positions.iter().map(|p| p.size).collect();
    let avg_prices: Vec<f64> = positions.iter().map(|p| p.avg_price).collect();
    let initial: Vec<f64> = positions.iter().map(|p| p.initial_value).collect();
    let current: Vec<f64> = positions.iter().map(|p| p.current_value).collect();
    let cash_pnl: Vec<f64> = positions.iter().map(|p| p.cash_pnl).collect();
    let pct_pnl: Vec<f64> = positions.iter().map(|p| p.percent_pnl).collect();
    let bought: Vec<f64> = positions.iter().map(|p| p.total_bought).collect();
    let realized: Vec<f64> = positions.iter().map(|p| p.realized_pnl).collect();
    let pct_realized: Vec<f64> = positions.iter().map(|p| p.percent_realized_pnl).collect();
    let cur_price: Vec<f64> = positions.iter().map(|p| p.cur_price).collect();
    let redeemable: Vec<bool> = positions.iter().map(|p| p.redeemable).collect();
    let mergeable: Vec<bool> = positions.iter().map(|p| p.mergeable).collect();
    let outcomes: Vec<Option<String>> = positions.iter().map(|p| p.outcome.clone()).collect();
    let outcome_idx: Vec<Option<i64>> = positions.iter().map(|p| p.outcome_index).collect();
    let end_dates: Vec<Option<String>> = positions.iter().map(|p| p.end_date.clone()).collect();
    let neg_risk: Vec<Option<bool>> = positions.iter().map(|p| p.negative_risk).collect();

    sqlx::query(
        "INSERT INTO positions
             (proxy_wallet, asset, captured_at, condition_id, size, avg_price,
              initial_value, current_value, cash_pnl, percent_pnl, total_bought,
              realized_pnl, percent_realized_pnl, cur_price, redeemable, mergeable,
              outcome, outcome_index, end_date, negative_risk)
         SELECT * FROM UNNEST(
             $1::text[], $2::text[], $3::timestamptz[], $4::text[], $5::float8[],
             $6::float8[], $7::float8[], $8::float8[], $9::float8[], $10::float8[],
             $11::float8[], $12::float8[], $13::float8[], $14::float8[], $15::bool[],
             $16::bool[], $17::text[], $18::int8[], $19::text[], $20::bool[])
         ON CONFLICT DO NOTHING",
    )
    .bind(&wallets).bind(&assets).bind(&ats).bind(&condition_ids).bind(&sizes)
    .bind(&avg_prices).bind(&initial).bind(&current).bind(&cash_pnl).bind(&pct_pnl)
    .bind(&bought).bind(&realized).bind(&pct_realized).bind(&cur_price).bind(&redeemable)
    .bind(&mergeable).bind(&outcomes).bind(&outcome_idx).bind(&end_dates).bind(&neg_risk)
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn insert_activity(
    pool: &PgPool,
    activities: &[Activity],
    scraped_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    if activities.is_empty() {
        return Ok(());
    }

    let usable: Vec<(&Activity, DateTime<Utc>)> = activities
        .iter()
        .filter_map(|a| ts_secs(a.timestamp).map(|ts| (a, ts)))
        .collect();
    if usable.is_empty() {
        return Ok(());
    }

    let hashes: Vec<String> = usable.iter().map(|(a, _)| a.transaction_hash.clone()).collect();
    let wallets: Vec<String> = usable.iter().map(|(a, _)| a.proxy_wallet.clone()).collect();
    let assets: Vec<String> = usable.iter().map(|(a, _)| a.asset.clone()).collect();
    let kinds: Vec<String> = usable.iter().map(|(a, _)| a.activity_type.clone()).collect();
    let tss: Vec<DateTime<Utc>> = usable.iter().map(|(_, ts)| *ts).collect();
    let condition_ids: Vec<String> = usable.iter().map(|(a, _)| a.condition_id.clone()).collect();
    let sizes: Vec<f64> = usable.iter().map(|(a, _)| a.size).collect();
    let usdc_sizes: Vec<f64> = usable.iter().map(|(a, _)| a.usdc_size).collect();
    let prices: Vec<f64> = usable.iter().map(|(a, _)| a.price).collect();
    let sides: Vec<Option<String>> = usable.iter().map(|(a, _)| a.side.clone()).collect();
    let outcomes: Vec<Option<String>> = usable.iter().map(|(a, _)| a.outcome.clone()).collect();
    let outcome_idx: Vec<Option<i64>> = usable.iter().map(|(a, _)| a.outcome_index).collect();
    let titles: Vec<Option<String>> = usable.iter().map(|(a, _)| a.title.clone()).collect();
    let slugs: Vec<Option<String>> = usable.iter().map(|(a, _)| a.slug.clone()).collect();
    let event_slugs: Vec<Option<String>> = usable.iter().map(|(a, _)| a.event_slug.clone()).collect();
    let scraped: Vec<DateTime<Utc>> = vec![scraped_at; usable.len()];

    sqlx::query(
        "INSERT INTO activity
             (transaction_hash, proxy_wallet, asset, activity_type, ts, condition_id,
              size, usdc_size, price, side, outcome, outcome_index, title, slug,
              event_slug, scraped_at)
         SELECT * FROM UNNEST(
             $1::text[], $2::text[], $3::text[], $4::text[], $5::timestamptz[],
             $6::text[], $7::float8[], $8::float8[], $9::float8[], $10::text[],
             $11::text[], $12::int8[], $13::text[], $14::text[], $15::text[],
             $16::timestamptz[])
         ON CONFLICT DO NOTHING",
    )
    .bind(&hashes).bind(&wallets).bind(&assets).bind(&kinds).bind(&tss)
    .bind(&condition_ids).bind(&sizes).bind(&usdc_sizes).bind(&prices).bind(&sides)
    .bind(&outcomes).bind(&outcome_idx).bind(&titles).bind(&slugs).bind(&event_slugs)
    .bind(&scraped)
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn insert_user_values(
    pool: &PgPool,
    values: &[UserValue],
    market: Option<&str>,
    captured_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    if values.is_empty() {
        return Ok(());
    }

    let wallets: Vec<String> = values.iter().map(|v| v.user.clone()).collect();
    let markets: Vec<String> = vec![market.unwrap_or("").to_string(); values.len()];
    let ats: Vec<DateTime<Utc>> = vec![captured_at; values.len()];
    let vals: Vec<f64> = values.iter().map(|v| v.value).collect();

    sqlx::query(
        "INSERT INTO user_values (proxy_wallet, market, captured_at, value)
         SELECT * FROM UNNEST($1::text[], $2::text[], $3::timestamptz[], $4::float8[])
         ON CONFLICT DO NOTHING",
    )
    .bind(&wallets).bind(&markets).bind(&ats).bind(&vals)
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn insert_holders(
    pool: &PgPool,
    holders: &[MarketHolders],
    captured_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let mut tokens: Vec<String> = Vec::new();
    let mut wallets: Vec<String> = Vec::new();
    let mut ats: Vec<DateTime<Utc>> = Vec::new();
    let mut assets: Vec<String> = Vec::new();
    let mut amounts: Vec<f64> = Vec::new();
    let mut outcome_idx: Vec<Option<i64>> = Vec::new();

    for mh in holders {
        for h in &mh.holders {
            tokens.push(mh.token.clone());
            wallets.push(h.proxy_wallet.clone());
            ats.push(captured_at);
            assets.push(h.asset.clone());
            amounts.push(h.amount);
            outcome_idx.push(h.outcome_index);
        }
    }

    if tokens.is_empty() {
        return Ok(());
    }

    sqlx::query(
        "INSERT INTO token_holders
             (token, proxy_wallet, captured_at, asset, amount, outcome_index)
         SELECT * FROM UNNEST(
             $1::text[], $2::text[], $3::timestamptz[], $4::text[], $5::float8[], $6::int8[])
         ON CONFLICT DO NOTHING",
    )
    .bind(&tokens).bind(&wallets).bind(&ats).bind(&assets).bind(&amounts).bind(&outcome_idx)
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn insert_user_pnl(
    pool: &PgPool,
    proxy_wallet: &str,
    points: &[UserPnlPoint],
) -> Result<(), sqlx::Error> {
    if points.is_empty() {
        return Ok(());
    }

    let usable: Vec<(DateTime<Utc>, f64)> = points
        .iter()
        .filter_map(|p| ts_secs(p.t).map(|t| (t, p.p)))
        .collect();

    let wallets: Vec<String> = vec![proxy_wallet.to_string(); usable.len()];
    let tss: Vec<DateTime<Utc>> = usable.iter().map(|(t, _)| *t).collect();
    let pnls: Vec<f64> = usable.iter().map(|(_, p)| *p).collect();

    sqlx::query(
        "INSERT INTO user_pnl (proxy_wallet, ts, pnl)
         SELECT * FROM UNNEST($1::text[], $2::timestamptz[], $3::float8[])
         ON CONFLICT (proxy_wallet, ts) DO NOTHING",
    )
    .bind(&wallets).bind(&tss).bind(&pnls)
    .execute(pool)
    .await
    .map(|_| ())
}
