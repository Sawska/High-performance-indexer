//! Log -> typed row.
//!
//! Anything that does not decode is dropped with a reason rather than guessed
//! at: a half-decoded fill is worse than a missing one, because it would quietly
//! corrupt the price series.

use super::abi;
use super::consts::{self, COLLATERAL_DECIMALS, OUTCOME_DECIMALS};
use super::rpc::Log;

#[derive(Debug, Clone, PartialEq)]
pub struct OrderFill {
    pub transaction_hash: String,
    pub log_index: i64,
    pub block_number: i64,
    pub exchange: String,
    pub exchange_version: i32,
    pub neg_risk: bool,
    pub event: &'static str,
    pub order_hash: String,
    pub maker: Option<String>,
    pub taker: Option<String>,
    pub side: Option<&'static str>,
    pub token_id: Option<String>,
    pub maker_asset_id: Option<String>,
    pub taker_asset_id: Option<String>,
    pub maker_amount_filled: f64,
    pub taker_amount_filled: f64,
    pub fee: Option<f64>,
    pub price: Option<f64>,
    pub size: Option<f64>,
    pub builder: Option<String>,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CtfPosition {
    pub transaction_hash: String,
    pub log_index: i64,
    pub block_number: i64,
    pub kind: &'static str,
    pub stakeholder: String,
    pub collateral_token: Option<String>,
    pub parent_collection_id: Option<String>,
    pub condition_id: String,
    pub partition_ids: Vec<String>,
    pub amount: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Redemption {
    pub transaction_hash: String,
    pub log_index: i64,
    pub block_number: i64,
    pub redeemer: String,
    pub collateral_token: Option<String>,
    pub parent_collection_id: Option<String>,
    pub condition_id: String,
    pub index_sets: Vec<String>,
    pub payout: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    pub condition_id: String,
    pub oracle: Option<String>,
    pub question_id: Option<String>,
    pub outcome_slot_count: Option<i64>,
    pub resolved: bool,
    pub payout_numerators: Option<Vec<String>>,
    pub transaction_hash: String,
    pub block_number: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TokenTransfer {
    pub transaction_hash: String,
    pub log_index: i64,
    pub item_index: i32,
    pub block_number: i64,
    pub contract: String,
    pub operator: Option<String>,
    pub from_address: String,
    pub to_address: String,
    pub token_id: String,
    pub amount: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CollateralTransfer {
    pub transaction_hash: String,
    pub log_index: i64,
    pub block_number: i64,
    pub token: String,
    pub symbol: Option<String>,
    pub from_address: String,
    pub to_address: String,
    pub amount: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Decoded {
    Fill(OrderFill),
    Position(CtfPosition),
    Redemption(Redemption),
    Condition(Condition),
    Tokens(Vec<TokenTransfer>),
    Collateral(CollateralTransfer),
}

/// Fields every log has, pulled out once.
struct Frame {
    tx: String,
    log_index: i64,
    block: i64,
    address: String,
    data: Vec<u8>,
}

fn frame(log: &Log) -> Option<Frame> {
    Some(Frame {
        tx: log.transaction_hash.to_lowercase(),
        log_index: log.index()? as i64,
        block: log.block()? as i64,
        address: log.address.to_lowercase(),
        data: abi::hex_bytes(&log.data)?,
    })
}

fn topic_bytes(log: &Log, i: usize) -> Option<Vec<u8>> {
    abi::hex_bytes(log.topics.get(i)?)
}

fn topic_address(log: &Log, i: usize) -> Option<String> {
    abi::address(&topic_bytes(log, i)?)
}

fn topic_hex32(log: &Log, i: usize) -> Option<String> {
    Some(abi::hex32(&topic_bytes(log, i)?))
}

fn ratio(numerator: f64, denominator: f64) -> Option<f64> {
    (denominator > 0.0).then_some(numerator / denominator)
}

/// `OrderFilled` / `OrdersMatched` from either exchange generation.
pub fn decode_exchange(log: &Log) -> Option<Decoded> {
    let f = frame(log)?;
    let ex = consts::exchange(&f.address)?;
    let topic0 = log.topic0()?;

    let is_filled = topic0 == *consts::TOPIC_ORDER_FILLED_V1
        || topic0 == *consts::TOPIC_ORDER_FILLED_V2;
    let is_matched = topic0 == *consts::TOPIC_ORDERS_MATCHED_V1
        || topic0 == *consts::TOPIC_ORDERS_MATCHED_V2;
    if !is_filled && !is_matched {
        return None;
    }

    // v1 identifies both sides of the swap and leaves the collateral side as
    // asset id 0; v2 states the side and a single token id outright.
    let v2 = topic0 == *consts::TOPIC_ORDER_FILLED_V2
        || topic0 == *consts::TOPIC_ORDERS_MATCHED_V2;

    let order_hash = topic_hex32(log, 1)?;
    let maker = topic_address(log, 2);
    // `OrderFilled` has a third indexed field (taker); `OrdersMatched` does not.
    let taker = if is_filled { topic_address(log, 3) } else { None };

    let mut fill = OrderFill {
        transaction_hash: f.tx,
        log_index: f.log_index,
        block_number: f.block,
        exchange: f.address,
        exchange_version: i32::from(ex.version),
        neg_risk: ex.neg_risk,
        event: if is_filled { "order_filled" } else { "orders_matched" },
        order_hash,
        maker,
        taker,
        side: None,
        token_id: None,
        maker_asset_id: None,
        taker_asset_id: None,
        maker_amount_filled: 0.0,
        taker_amount_filled: 0.0,
        fee: None,
        price: None,
        size: None,
        builder: None,
        metadata: None,
    };

    if v2 {
        // data: side, tokenId, makerAmountFilled, takerAmountFilled[, fee, builder, metadata]
        let side_raw = abi::word_u64(&f.data, 0)?;
        let token_id = abi::u256_decimal(abi::word(&f.data, 1)?);
        let maker_amount = abi::word(&f.data, 2)?;
        let taker_amount = abi::word(&f.data, 3)?;

        // `enum Side { BUY, SELL }`, from the maker's point of view.
        let buy = side_raw == 0;
        fill.side = Some(if buy { "BUY" } else { "SELL" });
        fill.token_id = Some(token_id);

        let (collateral, tokens) = if buy {
            (abi::scaled(maker_amount, COLLATERAL_DECIMALS), abi::scaled(taker_amount, OUTCOME_DECIMALS))
        } else {
            (abi::scaled(taker_amount, COLLATERAL_DECIMALS), abi::scaled(maker_amount, OUTCOME_DECIMALS))
        };
        fill.maker_amount_filled = abi::u256_f64(maker_amount) / 10f64.powi(6);
        fill.taker_amount_filled = abi::u256_f64(taker_amount) / 10f64.powi(6);
        fill.size = Some(tokens);
        fill.price = ratio(collateral, tokens);

        if is_filled {
            fill.fee = abi::word(&f.data, 4).map(|w| abi::scaled(w, COLLATERAL_DECIMALS));
            fill.builder = abi::word(&f.data, 5).map(abi::hex32);
            fill.metadata = abi::word(&f.data, 6).map(abi::hex32);
        }
    } else {
        // data: makerAssetId, takerAssetId, makerAmountFilled, takerAmountFilled[, fee]
        let maker_asset = abi::u256_decimal(abi::word(&f.data, 0)?);
        let taker_asset = abi::u256_decimal(abi::word(&f.data, 1)?);
        let maker_amount = abi::word(&f.data, 2)?;
        let taker_amount = abi::word(&f.data, 3)?;

        // Asset id 0 is the collateral leg, so whichever side is 0 tells us
        // which direction the maker traded.
        let maker_pays_cash = maker_asset == "0";
        fill.side = Some(if maker_pays_cash { "BUY" } else { "SELL" });
        fill.token_id = Some(if maker_pays_cash {
            taker_asset.clone()
        } else {
            maker_asset.clone()
        });

        let (collateral, tokens) = if maker_pays_cash {
            (abi::scaled(maker_amount, COLLATERAL_DECIMALS), abi::scaled(taker_amount, OUTCOME_DECIMALS))
        } else {
            (abi::scaled(taker_amount, COLLATERAL_DECIMALS), abi::scaled(maker_amount, OUTCOME_DECIMALS))
        };

        fill.maker_asset_id = Some(maker_asset);
        fill.taker_asset_id = Some(taker_asset);
        fill.maker_amount_filled = abi::u256_f64(maker_amount) / 10f64.powi(6);
        fill.taker_amount_filled = abi::u256_f64(taker_amount) / 10f64.powi(6);
        fill.size = Some(tokens);
        fill.price = ratio(collateral, tokens);

        if is_filled {
            fill.fee = abi::word(&f.data, 4).map(|w| abi::scaled(w, COLLATERAL_DECIMALS));
        }
    }

    Some(Decoded::Fill(fill))
}

/// The ConditionalTokens feed: lifecycle events plus ERC-1155 transfers.
pub fn decode_ctf(log: &Log) -> Option<Decoded> {
    let f = frame(log)?;
    let topic0 = log.topic0()?;

    if topic0 == *consts::TOPIC_POSITION_SPLIT || topic0 == *consts::TOPIC_POSITIONS_MERGE {
        let kind = if topic0 == *consts::TOPIC_POSITION_SPLIT { "split" } else { "merge" };
        // indexed: stakeholder, parentCollectionId, conditionId
        // data: collateralToken, partition[], amount
        return Some(Decoded::Position(CtfPosition {
            transaction_hash: f.tx,
            log_index: f.log_index,
            block_number: f.block,
            kind,
            stakeholder: topic_address(log, 1)?,
            collateral_token: abi::word(&f.data, 0).and_then(abi::address),
            parent_collection_id: topic_hex32(log, 2),
            condition_id: topic_hex32(log, 3)?,
            partition_ids: abi::u256_array(&f.data, 1).unwrap_or_default(),
            amount: abi::word(&f.data, 2).map_or(0.0, |w| abi::scaled(w, COLLATERAL_DECIMALS)),
        }));
    }

    if topic0 == *consts::TOPIC_PAYOUT_REDEMPTION {
        // indexed: redeemer, collateralToken, parentCollectionId
        // data: conditionId, indexSets[], payout
        return Some(Decoded::Redemption(Redemption {
            transaction_hash: f.tx,
            log_index: f.log_index,
            block_number: f.block,
            redeemer: topic_address(log, 1)?,
            collateral_token: topic_address(log, 2),
            parent_collection_id: topic_hex32(log, 3),
            condition_id: abi::word(&f.data, 0).map(abi::hex32)?,
            index_sets: abi::u256_array(&f.data, 1).unwrap_or_default(),
            payout: abi::word(&f.data, 2).map_or(0.0, |w| abi::scaled(w, COLLATERAL_DECIMALS)),
        }));
    }

    if topic0 == *consts::TOPIC_CONDITION_PREPARATION
        || topic0 == *consts::TOPIC_CONDITION_RESOLUTION
    {
        let resolved = topic0 == *consts::TOPIC_CONDITION_RESOLUTION;
        // indexed: conditionId, oracle, questionId; data: outcomeSlotCount[, payouts[]]
        return Some(Decoded::Condition(Condition {
            condition_id: topic_hex32(log, 1)?,
            oracle: topic_address(log, 2),
            question_id: topic_hex32(log, 3),
            outcome_slot_count: abi::word_u64(&f.data, 0).map(|v| v as i64),
            resolved,
            payout_numerators: resolved.then(|| abi::u256_array(&f.data, 1).unwrap_or_default()),
            transaction_hash: f.tx,
            block_number: f.block,
        }));
    }

    if topic0 == *consts::TOPIC_TRANSFER_SINGLE {
        // indexed: operator, from, to; data: id, value
        return Some(Decoded::Tokens(vec![TokenTransfer {
            transaction_hash: f.tx,
            log_index: f.log_index,
            item_index: 0,
            block_number: f.block,
            contract: f.address,
            operator: topic_address(log, 1),
            from_address: topic_address(log, 2)?,
            to_address: topic_address(log, 3)?,
            token_id: abi::u256_decimal(abi::word(&f.data, 0)?),
            amount: abi::scaled(abi::word(&f.data, 1)?, OUTCOME_DECIMALS),
        }]));
    }

    if topic0 == *consts::TOPIC_TRANSFER_BATCH {
        // indexed: operator, from, to; data: ids[], values[]
        let ids = abi::u256_array(&f.data, 0)?;
        let values = abi::u256_array(&f.data, 1)?;
        if ids.len() != values.len() {
            return None;
        }

        let operator = topic_address(log, 1);
        let from = topic_address(log, 2)?;
        let to = topic_address(log, 3)?;

        // One row per pair, so a batch is queryable the same way a single is.
        let rows = ids
            .into_iter()
            .zip(values)
            .enumerate()
            .map(|(i, (id, raw))| TokenTransfer {
                transaction_hash: f.tx.clone(),
                log_index: f.log_index,
                item_index: i as i32,
                block_number: f.block,
                contract: f.address.clone(),
                operator: operator.clone(),
                from_address: from.clone(),
                to_address: to.clone(),
                token_id: id,
                amount: raw.parse::<f64>().unwrap_or(0.0) / 10f64.powi(OUTCOME_DECIMALS as i32),
            })
            .collect();

        return Some(Decoded::Tokens(rows));
    }

    None
}

/// ERC-20 `Transfer` on a collateral token.
pub fn decode_collateral(log: &Log) -> Option<Decoded> {
    let f = frame(log)?;
    if log.topic0()? != *consts::TOPIC_ERC20_TRANSFER {
        return None;
    }

    Some(Decoded::Collateral(CollateralTransfer {
        transaction_hash: f.tx,
        log_index: f.log_index,
        block_number: f.block,
        symbol: consts::collateral_symbol(&f.address).map(str::to_string),
        token: f.address,
        from_address: topic_address(log, 1)?,
        to_address: topic_address(log, 2)?,
        amount: abi::scaled(abi::word(&f.data, 0)?, COLLATERAL_DECIMALS),
    }))
}

#[cfg(test)]
mod unit {
    use super::*;

    fn pad(body: &str) -> String {
        format!("{body:0>64}")
    }

    fn log(address: &str, topics: Vec<String>, words: &[&str]) -> Log {
        let data: String = words.iter().map(|w| pad(w)).collect();
        Log {
            address: address.to_string(),
            topics,
            data: format!("0x{data}"),
            block_number: "0x64".into(),
            transaction_hash: "0xTX".into(),
            log_index: "0x3".into(),
            removed: false,
        }
    }

    fn addr_topic(a: &str) -> String {
        format!("0x{}", pad(a.trim_start_matches("0x")))
    }

    #[test]
    fn v1_fill_reads_the_collateral_leg_from_asset_id_zero() {
        // maker pays 60 collateral for 100 outcome tokens => BUY at 0.60
        let l = log(
            consts::CTF_EXCHANGE_V1,
            vec![
                consts::TOPIC_ORDER_FILLED_V1.clone(),
                format!("0x{}", pad("aa")),
                addr_topic("0x1111111111111111111111111111111111111111"),
                addr_topic("0x2222222222222222222222222222222222222222"),
            ],
            &["0", "7b", "3938700", "5f5e100", "2710"],
        );

        let Some(Decoded::Fill(f)) = decode_exchange(&l) else {
            panic!("v1 OrderFilled should decode")
        };
        assert_eq!(f.side, Some("BUY"));
        assert_eq!(f.token_id.as_deref(), Some("123"), "taker asset is the token");
        assert_eq!(f.maker_asset_id.as_deref(), Some("0"));
        assert_eq!(f.size, Some(100.0));
        assert_eq!(f.price, Some(0.6));
        assert_eq!(f.fee, Some(0.01));
        assert_eq!(f.exchange_version, 1);
        assert!(!f.neg_risk);
        assert_eq!(f.event, "order_filled");
        assert_eq!(f.transaction_hash, "0xtx", "hashes are normalised");
        assert_eq!((f.log_index, f.block_number), (3, 100));
    }

    #[test]
    fn v1_fill_the_other_way_round_is_a_sell() {
        // maker gives 100 tokens, receives 60 collateral => SELL at 0.60
        let l = log(
            consts::CTF_EXCHANGE_V1,
            vec![
                consts::TOPIC_ORDER_FILLED_V1.clone(),
                format!("0x{}", pad("aa")),
                addr_topic("0x1111111111111111111111111111111111111111"),
                addr_topic("0x2222222222222222222222222222222222222222"),
            ],
            &["7b", "0", "5f5e100", "3938700", "0"],
        );

        let Some(Decoded::Fill(f)) = decode_exchange(&l) else {
            panic!("decodes")
        };
        assert_eq!(f.side, Some("SELL"));
        assert_eq!(f.token_id.as_deref(), Some("123"));
        assert_eq!(f.size, Some(100.0));
        assert_eq!(f.price, Some(0.6));
    }

    #[test]
    fn v2_fill_uses_the_explicit_side_and_token() {
        // side=SELL(1), tokenId=123, maker gives 100 tokens for 60 collateral
        let l = log(
            consts::CTF_EXCHANGE_V2,
            vec![
                consts::TOPIC_ORDER_FILLED_V2.clone(),
                format!("0x{}", pad("aa")),
                addr_topic("0x1111111111111111111111111111111111111111"),
                addr_topic("0x2222222222222222222222222222222222222222"),
            ],
            &["1", "7b", "5f5e100", "3938700", "2710", "bb", "cc"],
        );

        let Some(Decoded::Fill(f)) = decode_exchange(&l) else {
            panic!("v2 OrderFilled should decode")
        };
        assert_eq!(f.exchange_version, 2);
        assert_eq!(f.side, Some("SELL"));
        assert_eq!(f.token_id.as_deref(), Some("123"));
        assert_eq!(f.size, Some(100.0));
        assert_eq!(f.price, Some(0.6));
        assert_eq!(f.fee, Some(0.01));
        assert!(f.builder.as_deref().unwrap().ends_with("bb"));
        assert!(f.metadata.as_deref().unwrap().ends_with("cc"));
        assert_eq!(f.maker_asset_id, None, "v2 has no maker/taker asset ids");
    }

    #[test]
    fn a_v1_topic_on_a_v2_contract_still_decodes_as_v1() {
        // The two generations coexist; the topic decides the layout, not the
        // address, so historic logs stay readable after a contract upgrade.
        let l = log(
            consts::CTF_EXCHANGE_V2,
            vec![
                consts::TOPIC_ORDER_FILLED_V1.clone(),
                format!("0x{}", pad("aa")),
                addr_topic("0x1111111111111111111111111111111111111111"),
                addr_topic("0x2222222222222222222222222222222222222222"),
            ],
            &["0", "7b", "3938700", "5f5e100", "0"],
        );
        let Some(Decoded::Fill(f)) = decode_exchange(&l) else {
            panic!("decodes")
        };
        assert_eq!(f.maker_asset_id.as_deref(), Some("0"));
        assert_eq!(f.price, Some(0.6));
    }

    #[test]
    fn orders_matched_has_no_taker_topic() {
        let l = log(
            consts::NEG_RISK_CTF_EXCHANGE_V1,
            vec![
                consts::TOPIC_ORDERS_MATCHED_V1.clone(),
                format!("0x{}", pad("aa")),
                addr_topic("0x1111111111111111111111111111111111111111"),
            ],
            &["0", "7b", "3938700", "5f5e100"],
        );
        let Some(Decoded::Fill(f)) = decode_exchange(&l) else {
            panic!("decodes")
        };
        assert_eq!(f.event, "orders_matched");
        assert_eq!(f.taker, None);
        assert!(f.neg_risk);
        assert_eq!(f.fee, None, "OrdersMatched carries no fee word");
    }

    #[test]
    fn a_zero_denominator_yields_no_price_instead_of_infinity() {
        let l = log(
            consts::CTF_EXCHANGE_V1,
            vec![
                consts::TOPIC_ORDER_FILLED_V1.clone(),
                format!("0x{}", pad("aa")),
                addr_topic("0x1111111111111111111111111111111111111111"),
                addr_topic("0x2222222222222222222222222222222222222222"),
            ],
            &["0", "7b", "3938700", "0", "0"],
        );
        let Some(Decoded::Fill(f)) = decode_exchange(&l) else {
            panic!("decodes")
        };
        assert_eq!(f.price, None);
        assert_eq!(f.size, Some(0.0));
    }

    #[test]
    fn logs_from_an_unknown_contract_are_ignored() {
        let l = log(
            "0x000000000000000000000000000000000000dead",
            vec![consts::TOPIC_ORDER_FILLED_V1.clone(), format!("0x{}", pad("aa"))],
            &["0", "7b", "1", "1", "0"],
        );
        assert!(decode_exchange(&l).is_none());
    }

    #[test]
    fn position_split_carries_its_partition() {
        // data: collateralToken, offset(0x60), amount, [len=2, 1, 2]
        let l = log(
            consts::CONDITIONAL_TOKENS,
            vec![
                consts::TOPIC_POSITION_SPLIT.clone(),
                addr_topic("0x1111111111111111111111111111111111111111"),
                format!("0x{}", pad("0")),
                format!("0x{}", pad("c0")),
            ],
            &["2791bca1f2de4661ed88a30c99a7a9449aa84174", "60", "5f5e100", "2", "1", "2"],
        );

        let Some(Decoded::Position(p)) = decode_ctf(&l) else {
            panic!("PositionSplit should decode")
        };
        assert_eq!(p.kind, "split");
        assert_eq!(p.amount, 100.0);
        assert_eq!(p.partition_ids, vec!["1".to_string(), "2".to_string()]);
        assert_eq!(p.collateral_token.as_deref(), Some(consts::USDC_E));
        assert!(p.condition_id.ends_with("c0"));
    }

    #[test]
    fn redemption_reads_its_condition_from_the_data_not_the_topics() {
        // PayoutRedemption indexes parentCollectionId but not conditionId.
        let l = log(
            consts::CONDITIONAL_TOKENS,
            vec![
                consts::TOPIC_PAYOUT_REDEMPTION.clone(),
                addr_topic("0x1111111111111111111111111111111111111111"),
                addr_topic(consts::USDC_E),
                format!("0x{}", pad("0")),
            ],
            &["ab", "60", "5f5e100", "1", "1"],
        );

        let Some(Decoded::Redemption(r)) = decode_ctf(&l) else {
            panic!("PayoutRedemption should decode")
        };
        assert!(r.condition_id.ends_with("ab"));
        assert_eq!(r.collateral_token.as_deref(), Some(consts::USDC_E));
        assert_eq!(r.payout, 100.0);
        assert_eq!(r.index_sets, vec!["1".to_string()]);
    }

    #[test]
    fn condition_preparation_and_resolution_share_one_shape() {
        let prep = log(
            consts::CONDITIONAL_TOKENS,
            vec![
                consts::TOPIC_CONDITION_PREPARATION.clone(),
                format!("0x{}", pad("c1")),
                addr_topic("0x1111111111111111111111111111111111111111"),
                format!("0x{}", pad("q1")),
            ],
            &["2"],
        );
        let Some(Decoded::Condition(c)) = decode_ctf(&prep) else {
            panic!("decodes")
        };
        assert!(!c.resolved);
        assert_eq!(c.outcome_slot_count, Some(2));
        assert_eq!(c.payout_numerators, None);

        // resolution adds payoutNumerators after the slot count
        let res = log(
            consts::CONDITIONAL_TOKENS,
            vec![
                consts::TOPIC_CONDITION_RESOLUTION.clone(),
                format!("0x{}", pad("c1")),
                addr_topic("0x1111111111111111111111111111111111111111"),
                format!("0x{}", pad("q1")),
            ],
            &["2", "40", "2", "1", "0"],
        );
        let Some(Decoded::Condition(c)) = decode_ctf(&res) else {
            panic!("decodes")
        };
        assert!(c.resolved);
        assert_eq!(
            c.payout_numerators,
            Some(vec!["1".to_string(), "0".to_string()]),
            "YES paid out"
        );
    }

    #[test]
    fn transfer_batch_fans_out_to_one_row_per_pair() {
        // data: offset(0x40) ids, offset(0xa0) values, [2,1,2], [2, 1e8, 2e8]
        let l = log(
            consts::CONDITIONAL_TOKENS,
            vec![
                consts::TOPIC_TRANSFER_BATCH.clone(),
                addr_topic("0x1111111111111111111111111111111111111111"),
                addr_topic("0x2222222222222222222222222222222222222222"),
                addr_topic("0x3333333333333333333333333333333333333333"),
            ],
            &["40", "a0", "2", "7b", "7c", "2", "5f5e100", "bebc200"],
        );

        let Some(Decoded::Tokens(rows)) = decode_ctf(&l) else {
            panic!("TransferBatch should decode")
        };
        assert_eq!(rows.len(), 2);
        assert_eq!((rows[0].item_index, rows[1].item_index), (0, 1));
        assert_eq!(rows[0].token_id, "123");
        assert_eq!(rows[1].token_id, "124");
        assert_eq!(rows[0].amount, 100.0);
        assert_eq!(rows[1].amount, 200.0);
        assert!(rows.iter().all(|r| r.from_address.ends_with("2222")));
    }

    #[test]
    fn transfer_single_is_one_row() {
        let l = log(
            consts::CONDITIONAL_TOKENS,
            vec![
                consts::TOPIC_TRANSFER_SINGLE.clone(),
                addr_topic("0x1111111111111111111111111111111111111111"),
                addr_topic("0x2222222222222222222222222222222222222222"),
                addr_topic("0x3333333333333333333333333333333333333333"),
            ],
            &["7b", "5f5e100"],
        );
        let Some(Decoded::Tokens(rows)) = decode_ctf(&l) else {
            panic!("decodes")
        };
        assert_eq!(rows.len(), 1);
        assert_eq!((rows[0].token_id.as_str(), rows[0].amount), ("123", 100.0));
    }

    #[test]
    fn collateral_transfer_resolves_its_symbol() {
        let l = log(
            consts::PUSD,
            vec![
                consts::TOPIC_ERC20_TRANSFER.clone(),
                addr_topic("0x1111111111111111111111111111111111111111"),
                addr_topic(consts::CTF_EXCHANGE_V2),
            ],
            &["5f5e100"],
        );
        let Some(Decoded::Collateral(t)) = decode_collateral(&l) else {
            panic!("ERC-20 Transfer should decode")
        };
        assert_eq!(t.symbol.as_deref(), Some("pUSD"));
        assert_eq!(t.amount, 100.0);
        assert_eq!(t.to_address, consts::CTF_EXCHANGE_V2);
    }

    #[test]
    fn a_truncated_data_section_is_dropped_rather_than_guessed() {
        let l = log(
            consts::CTF_EXCHANGE_V1,
            vec![
                consts::TOPIC_ORDER_FILLED_V1.clone(),
                format!("0x{}", pad("aa")),
                addr_topic("0x1111111111111111111111111111111111111111"),
                addr_topic("0x2222222222222222222222222222222222222222"),
            ],
            &["0", "7b"], // missing the amount words
        );
        assert!(decode_exchange(&l).is_none());
    }
}
