//! Live checks against Polygon. `#[ignore]`d like the REST live tests; run with
//! `cargo test -- --ignored live_chain`.
//!
//! These exist because the log layouts cannot be verified offline: a wrong topic
//! hash or a misread word produces an empty result set rather than an error, and
//! only real logs prove the decoders line up with what the contracts emit.

use std::time::Duration;

use super::consts;
use super::events::{self, Decoded};
use super::rpc::Rpc;

fn rpc() -> Rpc {
    Rpc::new(
        std::env::var("CHAIN_RPC_URL").unwrap_or_else(|_| consts::POLYGON_RPC.to_string()),
        Duration::from_secs(30),
        3,
        Duration::from_millis(500),
    )
}

/// Walk back from the head until a range yields logs, so the test does not fail
/// merely because the most recent blocks happened to be quiet.
async fn find_logs(
    rpc: &Rpc,
    addresses: &[String],
    topics: &[String],
    span: u64,
    attempts: u32,
) -> Vec<super::rpc::Log> {
    let head = rpc.block_number().await.expect("head");
    let mut to = head - 64;

    for _ in 0..attempts {
        let from = to.saturating_sub(span);
        let logs = rpc.get_logs(from, to, addresses, topics).await.expect("getLogs");
        if !logs.is_empty() {
            return logs;
        }
        to = from.saturating_sub(1);
    }
    Vec::new()
}

#[tokio::test]
#[ignore]
async fn live_chain_head_and_block_agree() {
    let rpc = rpc();
    let head = rpc.block_number().await.expect("eth_blockNumber");
    assert!(head > 40_000_000, "polygon is well past block 40M, got {head}");

    let block = rpc.block(head - 64).await.expect("eth_getBlockByNumber").expect("block exists");
    assert_eq!(block.number, head - 64);
    assert!(block.hash.starts_with("0x") && block.hash.len() == 66);

    // Sanity-check the timestamp is recent rather than epoch garbage.
    let now = chrono::Utc::now().timestamp();
    assert!(
        (now - block.timestamp).abs() < 86_400,
        "block time {} is not within a day of now {now}",
        block.timestamp
    );
}

#[tokio::test]
#[ignore]
async fn live_chain_batching_returns_every_block_asked_for() {
    let rpc = rpc();
    let head = rpc.block_number().await.expect("head");
    let wanted: Vec<u64> = ((head - 84)..(head - 64)).collect();

    let blocks = rpc.blocks(&wanted, 10).await;
    assert_eq!(blocks.len(), wanted.len(), "a batch must not drop members");

    let mut numbers: Vec<u64> = blocks.iter().map(|b| b.number).collect();
    numbers.sort_unstable();
    assert_eq!(numbers, wanted, "batch results are re-sorted into request order");

    // Timestamps must increase with block number.
    let mut by_number = blocks.clone();
    by_number.sort_by_key(|b| b.number);
    for pair in by_number.windows(2) {
        assert!(pair[0].timestamp <= pair[1].timestamp);
    }
}

#[tokio::test]
#[ignore]
async fn live_chain_exchange_fills_decode() {
    let rpc = rpc();
    let addresses: Vec<String> = consts::EXCHANGES.iter().map(|e| e.address.to_string()).collect();
    let logs = find_logs(&rpc, &addresses, &consts::exchange_topics(), 200, 10).await;
    assert!(!logs.is_empty(), "no exchange logs found; addresses or topics are wrong");

    let mut fills = 0;
    let mut priced = 0;
    for log in &logs {
        let Some(Decoded::Fill(f)) = events::decode_exchange(log) else {
            continue;
        };
        fills += 1;

        assert!(matches!(f.exchange_version, 1 | 2));
        assert!(matches!(f.side, Some("BUY") | Some("SELL")));
        assert!(f.order_hash.len() == 66);

        if let Some(token) = &f.token_id {
            assert!(token.chars().all(|c| c.is_ascii_digit()), "token id is decimal");
        }
        if let Some(p) = f.price {
            // Every outcome token trades strictly inside [0, 1]; a price outside
            // that means the collateral leg was read from the wrong word.
            assert!((0.0..=1.0).contains(&p), "price {p} is outside [0,1] -- bad decode");
            priced += 1;
        }
    }

    assert!(fills > 0, "exchange logs decoded to zero fills");
    assert!(priced > 0, "no fill produced a price");
    println!("decoded {fills} fills from {} logs, {priced} priced", logs.len());
}

#[tokio::test]
#[ignore]
async fn live_chain_ctf_events_decode() {
    let rpc = rpc();
    let addresses = vec![consts::CONDITIONAL_TOKENS.to_string()];
    let topics = vec![
        consts::TOPIC_POSITION_SPLIT.clone(),
        consts::TOPIC_POSITIONS_MERGE.clone(),
        consts::TOPIC_PAYOUT_REDEMPTION.clone(),
        consts::TOPIC_CONDITION_PREPARATION.clone(),
        consts::TOPIC_CONDITION_RESOLUTION.clone(),
    ];

    let logs = find_logs(&rpc, &addresses, &topics, 500, 10).await;
    assert!(!logs.is_empty(), "no ConditionalTokens lifecycle logs found");

    let mut decoded = 0;
    for log in &logs {
        match events::decode_ctf(log) {
            Some(Decoded::Position(p)) => {
                assert_eq!(p.condition_id.len(), 66);
                assert!(!p.partition_ids.is_empty(), "a split/merge always has a partition");
                decoded += 1;
            }
            Some(Decoded::Redemption(r)) => {
                assert_eq!(r.condition_id.len(), 66);
                decoded += 1;
            }
            Some(Decoded::Condition(c)) => {
                assert_eq!(c.condition_id.len(), 66);
                if c.resolved {
                    let payouts = c.payout_numerators.as_ref().expect("resolution has payouts");
                    assert!(!payouts.is_empty());
                }
                decoded += 1;
            }
            _ => {}
        }
    }

    assert!(decoded > 0, "lifecycle logs decoded to nothing");
    println!("decoded {decoded} lifecycle events from {} logs", logs.len());
}

#[tokio::test]
#[ignore]
async fn live_chain_erc1155_transfers_decode() {
    let rpc = rpc();
    let addresses = vec![consts::CONDITIONAL_TOKENS.to_string()];
    let topics = vec![
        consts::TOPIC_TRANSFER_SINGLE.clone(),
        consts::TOPIC_TRANSFER_BATCH.clone(),
    ];

    let logs = find_logs(&rpc, &addresses, &topics, 100, 10).await;
    assert!(!logs.is_empty(), "no ERC-1155 transfers found on the CTF");

    let mut rows = 0;
    let mut batches = 0;
    for log in &logs {
        let Some(Decoded::Tokens(t)) = events::decode_ctf(log) else {
            continue;
        };
        if t.len() > 1 {
            batches += 1;
            // A batch must number its rows so the primary key stays unique.
            let indices: Vec<i32> = t.iter().map(|r| r.item_index).collect();
            assert_eq!(indices, (0..t.len() as i32).collect::<Vec<_>>());
        }
        for r in &t {
            assert!(r.token_id.chars().all(|c| c.is_ascii_digit()));
            assert!(r.from_address.len() == 42 && r.to_address.len() == 42);
            rows += 1;
        }
    }

    assert!(rows > 0, "no ERC-1155 rows decoded");
    println!("decoded {rows} transfer rows ({batches} batches) from {} logs", logs.len());
}

#[tokio::test]
#[ignore]
async fn live_chain_collateral_transfers_are_filtered_to_polymarket() {
    let rpc = rpc();
    let addresses: Vec<String> = consts::COLLATERALS.iter().map(|c| c.address.to_string()).collect();
    let topic = vec![consts::TOPIC_ERC20_TRANSFER.clone()];
    let watched: Vec<String> = consts::WATCHED_COUNTERPARTIES
        .iter()
        .map(|a| format!("0x{:0>64}", a.trim_start_matches("0x")))
        .collect();

    let head = rpc.block_number().await.expect("head");
    let (to, from) = (head - 64, head - 564);

    let mut total = 0;
    for position in [1_usize, 2] {
        let logs = rpc
            .get_logs_with_topic(from, to, &addresses, &topic, position, &watched)
            .await
            .expect("filtered getLogs");

        for log in &logs {
            let Some(Decoded::Collateral(t)) = events::decode_collateral(log) else {
                continue;
            };
            // The whole point of the topic filter: one side is always ours.
            let ours = consts::WATCHED_COUNTERPARTIES.contains(&t.from_address.as_str())
                || consts::WATCHED_COUNTERPARTIES.contains(&t.to_address.as_str());
            assert!(ours, "unfiltered transfer leaked through: {t:?}");
            assert!(consts::collateral_symbol(&t.token).is_some());
            total += 1;
        }
    }

    println!("decoded {total} collateral transfers touching Polymarket contracts");
}
