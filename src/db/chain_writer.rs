//! Batch writes for the on-chain feeds.
//!
//! Every row is keyed by `(transaction_hash, log_index)`, which is unique per
//! chain, so all inserts are `ON CONFLICT DO NOTHING`. That makes re-indexing a
//! block range idempotent -- which matters, because a crash mid-range or a
//! reorg rewind will replay blocks that are already stored.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;

use crate::venues::polymarket::onchain::events::{
    CollateralTransfer, Condition, CtfPosition, OrderFill, Redemption, TokenTransfer,
};
use crate::venues::polymarket::onchain::rpc::BlockMeta;

/// Block number -> timestamp, resolved separately from the logs themselves.
pub type BlockTimes = HashMap<i64, DateTime<Utc>>;

fn time_of(times: &BlockTimes, block: i64) -> Option<DateTime<Utc>> {
    times.get(&block).copied()
}

fn json_strings(v: &[String]) -> Value {
    Value::Array(v.iter().map(|s| Value::String(s.clone())).collect())
}

// ── cursor ──────────────────────────────────────────────────────────────────

pub async fn load_cursor(
    pool: &PgPool,
    name: &str,
) -> Result<Option<(i64, Option<String>)>, sqlx::Error> {
    let row: Option<(i64, Option<String>)> =
        sqlx::query_as("SELECT last_block, block_hash FROM chain_cursors WHERE name = $1")
            .bind(name)
            .fetch_optional(pool)
            .await?;
    Ok(row)
}

pub async fn save_cursor(
    pool: &PgPool,
    name: &str,
    last_block: i64,
    block_hash: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO chain_cursors (name, last_block, block_hash, updated_at)
         VALUES ($1, $2, $3, now())
         ON CONFLICT (name) DO UPDATE SET
             last_block = EXCLUDED.last_block,
             block_hash = EXCLUDED.block_hash,
             updated_at = EXCLUDED.updated_at",
    )
    .bind(name)
    .bind(last_block)
    .bind(block_hash)
    .execute(pool)
    .await?;
    Ok(())
}

/// Drop everything above `block`, so a reorged range can be re-indexed cleanly.
pub async fn rewind(pool: &PgPool, block: i64) -> Result<(), sqlx::Error> {
    for table in [
        "chain_order_fills",
        "chain_ctf_positions",
        "chain_ctf_redemptions",
        "chain_token_transfers",
        "chain_collateral_transfers",
        "chain_blocks",
    ] {
        let column = if table == "chain_blocks" { "number" } else { "block_number" };
        sqlx::query(&format!("DELETE FROM {table} WHERE {column} > $1"))
            .bind(block)
            .execute(pool)
            .await?;
    }

    // Conditions are one row per condition rather than per log, so the reorged
    // half is cleared column-wise instead of deleting the row.
    sqlx::query(
        "UPDATE chain_conditions
            SET resolved_block = NULL, resolved_at = NULL,
                resolved_tx = NULL, payout_numerators = NULL
          WHERE resolved_block > $1",
    )
    .bind(block)
    .execute(pool)
    .await?;
    sqlx::query("DELETE FROM chain_conditions WHERE prepared_block > $1")
        .bind(block)
        .execute(pool)
        .await?;

    Ok(())
}

// ── blocks ──────────────────────────────────────────────────────────────────

pub async fn block_times(pool: &PgPool, numbers: &[i64]) -> Result<BlockTimes, sqlx::Error> {
    if numbers.is_empty() {
        return Ok(BlockTimes::new());
    }
    let rows: Vec<(i64, DateTime<Utc>)> =
        sqlx::query_as("SELECT number, ts FROM chain_blocks WHERE number = ANY($1::bigint[])")
            .bind(numbers)
            .fetch_all(pool)
            .await?;
    Ok(rows.into_iter().collect())
}

pub async fn insert_blocks(pool: &PgPool, blocks: &[BlockMeta]) -> Result<(), sqlx::Error> {
    if blocks.is_empty() {
        return Ok(());
    }

    let mut seen = HashSet::new();
    let uniq: Vec<&BlockMeta> = blocks.iter().filter(|b| seen.insert(b.number)).collect();

    let numbers: Vec<i64> = uniq.iter().map(|b| b.number as i64).collect();
    let hashes: Vec<String> = uniq.iter().map(|b| b.hash.to_lowercase()).collect();
    let tss: Vec<DateTime<Utc>> = uniq
        .iter()
        .map(|b| DateTime::from_timestamp(b.timestamp, 0).unwrap_or_else(Utc::now))
        .collect();

    sqlx::query(
        "INSERT INTO chain_blocks (number, hash, ts)
         SELECT * FROM UNNEST($1::bigint[], $2::text[], $3::timestamptz[])
         ON CONFLICT (number) DO UPDATE SET hash = EXCLUDED.hash, ts = EXCLUDED.ts",
    )
    .bind(&numbers)
    .bind(&hashes)
    .bind(&tss)
    .execute(pool)
    .await?;

    Ok(())
}

// ── events ──────────────────────────────────────────────────────────────────

pub async fn insert_fills(
    pool: &PgPool,
    rows: &[OrderFill],
    times: &BlockTimes,
) -> Result<u64, sqlx::Error> {
    if rows.is_empty() {
        return Ok(0);
    }

    let txs: Vec<String> = rows.iter().map(|r| r.transaction_hash.clone()).collect();
    let idxs: Vec<i64> = rows.iter().map(|r| r.log_index).collect();
    let blocks: Vec<i64> = rows.iter().map(|r| r.block_number).collect();
    let tss: Vec<Option<DateTime<Utc>>> =
        rows.iter().map(|r| time_of(times, r.block_number)).collect();
    let exchanges: Vec<String> = rows.iter().map(|r| r.exchange.clone()).collect();
    let versions: Vec<i32> = rows.iter().map(|r| r.exchange_version).collect();
    let neg: Vec<bool> = rows.iter().map(|r| r.neg_risk).collect();
    let events: Vec<String> = rows.iter().map(|r| r.event.to_string()).collect();
    let hashes: Vec<String> = rows.iter().map(|r| r.order_hash.clone()).collect();
    let makers: Vec<Option<String>> = rows.iter().map(|r| r.maker.clone()).collect();
    let takers: Vec<Option<String>> = rows.iter().map(|r| r.taker.clone()).collect();
    let sides: Vec<Option<String>> = rows.iter().map(|r| r.side.map(str::to_string)).collect();
    let tokens: Vec<Option<String>> = rows.iter().map(|r| r.token_id.clone()).collect();
    let maker_assets: Vec<Option<String>> = rows.iter().map(|r| r.maker_asset_id.clone()).collect();
    let taker_assets: Vec<Option<String>> = rows.iter().map(|r| r.taker_asset_id.clone()).collect();
    let maker_amounts: Vec<f64> = rows.iter().map(|r| r.maker_amount_filled).collect();
    let taker_amounts: Vec<f64> = rows.iter().map(|r| r.taker_amount_filled).collect();
    let fees: Vec<Option<f64>> = rows.iter().map(|r| r.fee).collect();
    let prices: Vec<Option<f64>> = rows.iter().map(|r| r.price).collect();
    let sizes: Vec<Option<f64>> = rows.iter().map(|r| r.size).collect();
    let builders: Vec<Option<String>> = rows.iter().map(|r| r.builder.clone()).collect();
    let metadatas: Vec<Option<String>> = rows.iter().map(|r| r.metadata.clone()).collect();

    let res = sqlx::query(
        "INSERT INTO chain_order_fills
             (transaction_hash, log_index, block_number, block_time, exchange,
              exchange_version, neg_risk, event, order_hash, maker, taker, side,
              token_id, maker_asset_id, taker_asset_id, maker_amount_filled,
              taker_amount_filled, fee, price, size, builder, metadata)
         SELECT * FROM UNNEST(
             $1::text[], $2::bigint[], $3::bigint[], $4::timestamptz[], $5::text[],
             $6::int[], $7::bool[], $8::text[], $9::text[], $10::text[], $11::text[],
             $12::text[], $13::text[], $14::text[], $15::text[], $16::float8[],
             $17::float8[], $18::float8[], $19::float8[], $20::float8[], $21::text[],
             $22::text[])
         ON CONFLICT (transaction_hash, log_index) DO NOTHING",
    )
    .bind(&txs).bind(&idxs).bind(&blocks).bind(&tss).bind(&exchanges)
    .bind(&versions).bind(&neg).bind(&events).bind(&hashes).bind(&makers)
    .bind(&takers).bind(&sides).bind(&tokens).bind(&maker_assets)
    .bind(&taker_assets).bind(&maker_amounts).bind(&taker_amounts).bind(&fees)
    .bind(&prices).bind(&sizes).bind(&builders).bind(&metadatas)
    .execute(pool)
    .await?;

    Ok(res.rows_affected())
}

pub async fn insert_positions(
    pool: &PgPool,
    rows: &[CtfPosition],
    times: &BlockTimes,
) -> Result<u64, sqlx::Error> {
    if rows.is_empty() {
        return Ok(0);
    }

    let txs: Vec<String> = rows.iter().map(|r| r.transaction_hash.clone()).collect();
    let idxs: Vec<i64> = rows.iter().map(|r| r.log_index).collect();
    let blocks: Vec<i64> = rows.iter().map(|r| r.block_number).collect();
    let tss: Vec<Option<DateTime<Utc>>> =
        rows.iter().map(|r| time_of(times, r.block_number)).collect();
    let kinds: Vec<String> = rows.iter().map(|r| r.kind.to_string()).collect();
    let holders: Vec<String> = rows.iter().map(|r| r.stakeholder.clone()).collect();
    let collaterals: Vec<Option<String>> =
        rows.iter().map(|r| r.collateral_token.clone()).collect();
    let parents: Vec<Option<String>> =
        rows.iter().map(|r| r.parent_collection_id.clone()).collect();
    let conditions: Vec<String> = rows.iter().map(|r| r.condition_id.clone()).collect();
    let partitions: Vec<Value> = rows.iter().map(|r| json_strings(&r.partition_ids)).collect();
    let amounts: Vec<f64> = rows.iter().map(|r| r.amount).collect();

    let res = sqlx::query(
        "INSERT INTO chain_ctf_positions
             (transaction_hash, log_index, block_number, block_time, kind,
              stakeholder, collateral_token, parent_collection_id, condition_id,
              partition_ids, amount)
         SELECT * FROM UNNEST(
             $1::text[], $2::bigint[], $3::bigint[], $4::timestamptz[], $5::text[],
             $6::text[], $7::text[], $8::text[], $9::text[], $10::jsonb[], $11::float8[])
         ON CONFLICT (transaction_hash, log_index) DO NOTHING",
    )
    .bind(&txs).bind(&idxs).bind(&blocks).bind(&tss).bind(&kinds)
    .bind(&holders).bind(&collaterals).bind(&parents).bind(&conditions)
    .bind(&partitions).bind(&amounts)
    .execute(pool)
    .await?;

    Ok(res.rows_affected())
}

pub async fn insert_redemptions(
    pool: &PgPool,
    rows: &[Redemption],
    times: &BlockTimes,
) -> Result<u64, sqlx::Error> {
    if rows.is_empty() {
        return Ok(0);
    }

    let txs: Vec<String> = rows.iter().map(|r| r.transaction_hash.clone()).collect();
    let idxs: Vec<i64> = rows.iter().map(|r| r.log_index).collect();
    let blocks: Vec<i64> = rows.iter().map(|r| r.block_number).collect();
    let tss: Vec<Option<DateTime<Utc>>> =
        rows.iter().map(|r| time_of(times, r.block_number)).collect();
    let redeemers: Vec<String> = rows.iter().map(|r| r.redeemer.clone()).collect();
    let collaterals: Vec<Option<String>> =
        rows.iter().map(|r| r.collateral_token.clone()).collect();
    let parents: Vec<Option<String>> =
        rows.iter().map(|r| r.parent_collection_id.clone()).collect();
    let conditions: Vec<String> = rows.iter().map(|r| r.condition_id.clone()).collect();
    let index_sets: Vec<Value> = rows.iter().map(|r| json_strings(&r.index_sets)).collect();
    let payouts: Vec<f64> = rows.iter().map(|r| r.payout).collect();

    let res = sqlx::query(
        "INSERT INTO chain_ctf_redemptions
             (transaction_hash, log_index, block_number, block_time, redeemer,
              collateral_token, parent_collection_id, condition_id, index_sets, payout)
         SELECT * FROM UNNEST(
             $1::text[], $2::bigint[], $3::bigint[], $4::timestamptz[], $5::text[],
             $6::text[], $7::text[], $8::text[], $9::jsonb[], $10::float8[])
         ON CONFLICT (transaction_hash, log_index) DO NOTHING",
    )
    .bind(&txs).bind(&idxs).bind(&blocks).bind(&tss).bind(&redeemers)
    .bind(&collaterals).bind(&parents).bind(&conditions).bind(&index_sets)
    .bind(&payouts)
    .execute(pool)
    .await?;

    Ok(res.rows_affected())
}

/// Preparation and resolution both land on one row per condition.
///
/// Rows are merged in Rust first: Postgres rejects an `ON CONFLICT DO UPDATE`
/// that touches the same key twice in a single statement, and a range that
/// contains both events for one condition would do exactly that.
pub async fn upsert_conditions(
    pool: &PgPool,
    rows: &[Condition],
    times: &BlockTimes,
) -> Result<u64, sqlx::Error> {
    if rows.is_empty() {
        return Ok(0);
    }

    struct Merged {
        oracle: Option<String>,
        question_id: Option<String>,
        slots: Option<i64>,
        prepared: Option<(i64, Option<DateTime<Utc>>, String)>,
        resolved: Option<(i64, Option<DateTime<Utc>>, String, Value)>,
    }

    let mut by_condition: HashMap<String, Merged> = HashMap::new();
    for r in rows {
        let entry = by_condition.entry(r.condition_id.clone()).or_insert(Merged {
            oracle: None,
            question_id: None,
            slots: None,
            prepared: None,
            resolved: None,
        });
        entry.oracle = entry.oracle.take().or_else(|| r.oracle.clone());
        entry.question_id = entry.question_id.take().or_else(|| r.question_id.clone());
        entry.slots = entry.slots.or(r.outcome_slot_count);

        let at = time_of(times, r.block_number);
        if r.resolved {
            entry.resolved = Some((
                r.block_number,
                at,
                r.transaction_hash.clone(),
                json_strings(r.payout_numerators.as_deref().unwrap_or_default()),
            ));
        } else {
            entry.prepared = Some((r.block_number, at, r.transaction_hash.clone()));
        }
    }

    let mut ids = Vec::new();
    let mut oracles = Vec::new();
    let mut questions = Vec::new();
    let mut slots = Vec::new();
    let mut prep_blocks = Vec::new();
    let mut prep_ats = Vec::new();
    let mut prep_txs = Vec::new();
    let mut res_blocks = Vec::new();
    let mut res_ats = Vec::new();
    let mut res_txs = Vec::new();
    let mut payouts = Vec::new();

    for (id, m) in by_condition {
        ids.push(id);
        oracles.push(m.oracle);
        questions.push(m.question_id);
        slots.push(m.slots);
        prep_blocks.push(m.prepared.as_ref().map(|p| p.0));
        prep_ats.push(m.prepared.as_ref().and_then(|p| p.1));
        prep_txs.push(m.prepared.as_ref().map(|p| p.2.clone()));
        res_blocks.push(m.resolved.as_ref().map(|r| r.0));
        res_ats.push(m.resolved.as_ref().and_then(|r| r.1));
        res_txs.push(m.resolved.as_ref().map(|r| r.2.clone()));
        payouts.push(m.resolved.as_ref().map(|r| r.3.clone()));
    }

    // COALESCE keeps whichever half of the lifecycle is already stored: a
    // resolution arriving later must not blank out the preparation columns.
    let res = sqlx::query(
        "INSERT INTO chain_conditions
             (condition_id, oracle, question_id, outcome_slot_count,
              prepared_block, prepared_at, prepared_tx,
              resolved_block, resolved_at, resolved_tx, payout_numerators)
         SELECT * FROM UNNEST(
             $1::text[], $2::text[], $3::text[], $4::bigint[], $5::bigint[],
             $6::timestamptz[], $7::text[], $8::bigint[], $9::timestamptz[],
             $10::text[], $11::jsonb[])
         ON CONFLICT (condition_id) DO UPDATE SET
             oracle             = COALESCE(EXCLUDED.oracle, chain_conditions.oracle),
             question_id        = COALESCE(EXCLUDED.question_id, chain_conditions.question_id),
             outcome_slot_count = COALESCE(EXCLUDED.outcome_slot_count, chain_conditions.outcome_slot_count),
             prepared_block     = COALESCE(EXCLUDED.prepared_block, chain_conditions.prepared_block),
             prepared_at        = COALESCE(EXCLUDED.prepared_at, chain_conditions.prepared_at),
             prepared_tx        = COALESCE(EXCLUDED.prepared_tx, chain_conditions.prepared_tx),
             resolved_block     = COALESCE(EXCLUDED.resolved_block, chain_conditions.resolved_block),
             resolved_at        = COALESCE(EXCLUDED.resolved_at, chain_conditions.resolved_at),
             resolved_tx        = COALESCE(EXCLUDED.resolved_tx, chain_conditions.resolved_tx),
             payout_numerators  = COALESCE(EXCLUDED.payout_numerators, chain_conditions.payout_numerators)",
    )
    .bind(&ids).bind(&oracles).bind(&questions).bind(&slots)
    .bind(&prep_blocks).bind(&prep_ats).bind(&prep_txs)
    .bind(&res_blocks).bind(&res_ats).bind(&res_txs).bind(&payouts)
    .execute(pool)
    .await?;

    Ok(res.rows_affected())
}

pub async fn insert_token_transfers(
    pool: &PgPool,
    rows: &[TokenTransfer],
    times: &BlockTimes,
) -> Result<u64, sqlx::Error> {
    if rows.is_empty() {
        return Ok(0);
    }

    let txs: Vec<String> = rows.iter().map(|r| r.transaction_hash.clone()).collect();
    let idxs: Vec<i64> = rows.iter().map(|r| r.log_index).collect();
    let items: Vec<i32> = rows.iter().map(|r| r.item_index).collect();
    let blocks: Vec<i64> = rows.iter().map(|r| r.block_number).collect();
    let tss: Vec<Option<DateTime<Utc>>> =
        rows.iter().map(|r| time_of(times, r.block_number)).collect();
    let contracts: Vec<String> = rows.iter().map(|r| r.contract.clone()).collect();
    let operators: Vec<Option<String>> = rows.iter().map(|r| r.operator.clone()).collect();
    let froms: Vec<String> = rows.iter().map(|r| r.from_address.clone()).collect();
    let tos: Vec<String> = rows.iter().map(|r| r.to_address.clone()).collect();
    let tokens: Vec<String> = rows.iter().map(|r| r.token_id.clone()).collect();
    let amounts: Vec<f64> = rows.iter().map(|r| r.amount).collect();

    let res = sqlx::query(
        "INSERT INTO chain_token_transfers
             (transaction_hash, log_index, item_index, block_number, block_time,
              contract, operator, from_address, to_address, token_id, amount)
         SELECT * FROM UNNEST(
             $1::text[], $2::bigint[], $3::int[], $4::bigint[], $5::timestamptz[],
             $6::text[], $7::text[], $8::text[], $9::text[], $10::text[], $11::float8[])
         ON CONFLICT (transaction_hash, log_index, item_index) DO NOTHING",
    )
    .bind(&txs).bind(&idxs).bind(&items).bind(&blocks).bind(&tss)
    .bind(&contracts).bind(&operators).bind(&froms).bind(&tos)
    .bind(&tokens).bind(&amounts)
    .execute(pool)
    .await?;

    Ok(res.rows_affected())
}

pub async fn insert_collateral_transfers(
    pool: &PgPool,
    rows: &[CollateralTransfer],
    times: &BlockTimes,
) -> Result<u64, sqlx::Error> {
    if rows.is_empty() {
        return Ok(0);
    }

    // The same transfer can arrive twice in one range: the from-side and
    // to-side queries both match when Polymarket contracts sit on both ends.
    let mut seen = HashSet::new();
    let uniq: Vec<&CollateralTransfer> = rows
        .iter()
        .filter(|r| seen.insert((r.transaction_hash.clone(), r.log_index)))
        .collect();

    let txs: Vec<String> = uniq.iter().map(|r| r.transaction_hash.clone()).collect();
    let idxs: Vec<i64> = uniq.iter().map(|r| r.log_index).collect();
    let blocks: Vec<i64> = uniq.iter().map(|r| r.block_number).collect();
    let tss: Vec<Option<DateTime<Utc>>> =
        uniq.iter().map(|r| time_of(times, r.block_number)).collect();
    let tokens: Vec<String> = uniq.iter().map(|r| r.token.clone()).collect();
    let symbols: Vec<Option<String>> = uniq.iter().map(|r| r.symbol.clone()).collect();
    let froms: Vec<String> = uniq.iter().map(|r| r.from_address.clone()).collect();
    let tos: Vec<String> = uniq.iter().map(|r| r.to_address.clone()).collect();
    let amounts: Vec<f64> = uniq.iter().map(|r| r.amount).collect();

    let res = sqlx::query(
        "INSERT INTO chain_collateral_transfers
             (transaction_hash, log_index, block_number, block_time, token,
              symbol, from_address, to_address, amount)
         SELECT * FROM UNNEST(
             $1::text[], $2::bigint[], $3::bigint[], $4::timestamptz[], $5::text[],
             $6::text[], $7::text[], $8::text[], $9::float8[])
         ON CONFLICT (transaction_hash, log_index) DO NOTHING",
    )
    .bind(&txs).bind(&idxs).bind(&blocks).bind(&tss).bind(&tokens)
    .bind(&symbols).bind(&froms).bind(&tos).bind(&amounts)
    .execute(pool)
    .await?;

    Ok(res.rows_affected())
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn json_strings_keeps_ids_as_text() {
        // Partition ids are uint256; as JSON numbers they would lose precision.
        let v = json_strings(&["1".into(), "115792089237316195423570985008687907853269984665640564039457584007913129639935".into()]);
        assert!(v[0].is_string());
        assert_eq!(v[1].as_str().unwrap().len(), 78);
    }

    #[test]
    fn missing_block_times_stay_null_rather_than_defaulting() {
        let mut times = BlockTimes::new();
        times.insert(10, Utc::now());
        assert!(time_of(&times, 10).is_some());
        assert!(time_of(&times, 11).is_none());
    }
}
