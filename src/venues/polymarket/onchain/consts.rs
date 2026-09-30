//! Polygon mainnet addresses and event signatures.
//!
//! Addresses are stored lower-cased because `eth_getLogs` returns them that way
//! and every comparison here is a plain string compare. Topic hashes are derived
//! from the signature strings at first use rather than hard-coded, so a typo in a
//! literal can never silently produce a filter that matches nothing.

use std::sync::LazyLock;

use sha3::{Digest, Keccak256};

/// The long-standing `polygon-rpc.com` now answers 401 without an API key,
/// so the default is a genuinely keyless endpoint. Override with CHAIN_RPC_URL.
pub const POLYGON_RPC: &str = "https://polygon-bor-rpc.publicnode.com";

pub const CTF_EXCHANGE_V1: &str = "0x4bfb41d5b3570defd03c39a9a4d8de6bd8b8982e";
pub const NEG_RISK_CTF_EXCHANGE_V1: &str = "0xc5d563a36ae78145c45a50134d48a1215220f80a";
pub const CTF_EXCHANGE_V2: &str = "0xe111180000d2663c0091e4f400237545b87b996b";
pub const NEG_RISK_CTF_EXCHANGE_V2: &str = "0xe2222d279d744050d28e00520010520000310f59";

pub const CONDITIONAL_TOKENS: &str = "0x4d97dcd97ec945f40cf65f87097ace5ea0476045";
pub const NEG_RISK_ADAPTER: &str = "0xd91e80cf2e7be2e162c6513ced06f1dd0da35296";
pub const NEG_RISK_VAULT: &str = "0x7f67327e88c258932d7d8f72950be0d46975e11d";
pub const NEG_RISK_FEE_MODULE: &str = "0x78769d50be1763ed1ca0d5e878d93f05aabff29e";

/// Current collateral. Replaced USDC.e; both are watched by default so history
/// spanning the migration stays complete.
pub const PUSD: &str = "0xc011a7e12a19f7b1f670d46f03b03f3342e82dfb";
pub const USDC_E: &str = "0x2791bca1f2de4661ed88a30c99a7a9449aa84174";

/// Collateral and outcome tokens are both 6-decimal, which is what makes an
/// outcome token redeemable 1:1 for collateral.
pub const COLLATERAL_DECIMALS: u32 = 6;
pub const OUTCOME_DECIMALS: u32 = 6;

pub struct Exchange {
    pub address: &'static str,
    pub version: u8,
    pub neg_risk: bool,
}

pub const EXCHANGES: &[Exchange] = &[
    Exchange { address: CTF_EXCHANGE_V1,          version: 1, neg_risk: false },
    Exchange { address: NEG_RISK_CTF_EXCHANGE_V1, version: 1, neg_risk: true  },
    Exchange { address: CTF_EXCHANGE_V2,          version: 2, neg_risk: false },
    Exchange { address: NEG_RISK_CTF_EXCHANGE_V2, version: 2, neg_risk: true  },
];

pub fn exchange(address: &str) -> Option<&'static Exchange> {
    EXCHANGES.iter().find(|e| e.address == address)
}

pub struct Collateral {
    pub address: &'static str,
    pub symbol: &'static str,
}

pub const COLLATERALS: &[Collateral] = &[
    Collateral { address: PUSD,   symbol: "pUSD"   },
    Collateral { address: USDC_E, symbol: "USDC.e" },
];

pub fn collateral_symbol(address: &str) -> Option<&'static str> {
    COLLATERALS
        .iter()
        .find(|c| c.address == address)
        .map(|c| c.symbol)
}

/// Counterparties whose collateral flows we care about. An ERC-20 `Transfer` is
/// kept only when one side is in this list -- the unfiltered pUSD/USDC.e feed is
/// all of Polygon and would swamp the database.
pub const WATCHED_COUNTERPARTIES: &[&str] = &[
    CTF_EXCHANGE_V1,
    NEG_RISK_CTF_EXCHANGE_V1,
    CTF_EXCHANGE_V2,
    NEG_RISK_CTF_EXCHANGE_V2,
    CONDITIONAL_TOKENS,
    NEG_RISK_ADAPTER,
    NEG_RISK_VAULT,
    NEG_RISK_FEE_MODULE,
];

// ── event signatures ────────────────────────────────────────────────────────
//
// v1 and v2 of the exchange emit *different* `OrderFilled`/`OrdersMatched`: v1
// names both sides of the swap (makerAssetId/takerAssetId, one of which is 0 for
// collateral), v2 names the side and a single tokenId and adds builder/metadata.
// Different signature => different topic0 => different data layout.

pub const SIG_ORDER_FILLED_V1: &str =
    "OrderFilled(bytes32,address,address,uint256,uint256,uint256,uint256,uint256)";
pub const SIG_ORDERS_MATCHED_V1: &str =
    "OrdersMatched(bytes32,address,uint256,uint256,uint256,uint256)";
pub const SIG_ORDER_FILLED_V2: &str =
    "OrderFilled(bytes32,address,address,uint8,uint256,uint256,uint256,uint256,bytes32,bytes32)";
pub const SIG_ORDERS_MATCHED_V2: &str =
    "OrdersMatched(bytes32,address,uint8,uint256,uint256,uint256)";

pub const SIG_POSITION_SPLIT: &str =
    "PositionSplit(address,address,bytes32,bytes32,uint256[],uint256)";
pub const SIG_POSITIONS_MERGE: &str =
    "PositionsMerge(address,address,bytes32,bytes32,uint256[],uint256)";
pub const SIG_PAYOUT_REDEMPTION: &str =
    "PayoutRedemption(address,address,bytes32,bytes32,uint256[],uint256)";
pub const SIG_CONDITION_PREPARATION: &str = "ConditionPreparation(bytes32,address,bytes32,uint256)";
pub const SIG_CONDITION_RESOLUTION: &str =
    "ConditionResolution(bytes32,address,bytes32,uint256,uint256[])";

pub const SIG_TRANSFER_SINGLE: &str = "TransferSingle(address,address,address,uint256,uint256)";
pub const SIG_TRANSFER_BATCH: &str = "TransferBatch(address,address,address,uint256[],uint256[])";
pub const SIG_ERC20_TRANSFER: &str = "Transfer(address,address,uint256)";

/// keccak256 of an event signature: the value that lands in `topics[0]`.
pub fn topic0(signature: &str) -> String {
    format!("0x{}", hex::encode(Keccak256::digest(signature.as_bytes())))
}

macro_rules! topic {
    ($name:ident, $sig:expr) => {
        pub static $name: LazyLock<String> = LazyLock::new(|| topic0($sig));
    };
}

topic!(TOPIC_ORDER_FILLED_V1, SIG_ORDER_FILLED_V1);
topic!(TOPIC_ORDERS_MATCHED_V1, SIG_ORDERS_MATCHED_V1);
topic!(TOPIC_ORDER_FILLED_V2, SIG_ORDER_FILLED_V2);
topic!(TOPIC_ORDERS_MATCHED_V2, SIG_ORDERS_MATCHED_V2);
topic!(TOPIC_POSITION_SPLIT, SIG_POSITION_SPLIT);
topic!(TOPIC_POSITIONS_MERGE, SIG_POSITIONS_MERGE);
topic!(TOPIC_PAYOUT_REDEMPTION, SIG_PAYOUT_REDEMPTION);
topic!(TOPIC_CONDITION_PREPARATION, SIG_CONDITION_PREPARATION);
topic!(TOPIC_CONDITION_RESOLUTION, SIG_CONDITION_RESOLUTION);
topic!(TOPIC_TRANSFER_SINGLE, SIG_TRANSFER_SINGLE);
topic!(TOPIC_TRANSFER_BATCH, SIG_TRANSFER_BATCH);
topic!(TOPIC_ERC20_TRANSFER, SIG_ERC20_TRANSFER);

pub fn exchange_topics() -> Vec<String> {
    vec![
        TOPIC_ORDER_FILLED_V1.clone(),
        TOPIC_ORDERS_MATCHED_V1.clone(),
        TOPIC_ORDER_FILLED_V2.clone(),
        TOPIC_ORDERS_MATCHED_V2.clone(),
    ]
}

#[cfg(test)]
mod unit {
    use super::*;

    /// Guards the keccak plumbing against the two topic0 values that are
    /// published in the ERC-20 / ERC-1155 standards. If `topic0` were wrong,
    /// every log filter would silently match nothing.
    #[test]
    fn well_known_topics_match_the_standards() {
        assert_eq!(
            topic0(SIG_ERC20_TRANSFER),
            "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
        );
        assert_eq!(
            topic0(SIG_TRANSFER_SINGLE),
            "0xc3d58168c5ae7397731d063d5bbf3d657854427343f4c083240f7aacaa2d0f62"
        );
        assert_eq!(
            topic0(SIG_TRANSFER_BATCH),
            "0x4a39dc06d4c0dbc64b70af90fd698a233a518aa5d07e595d983b8c0526c8f7fb"
        );
        assert_eq!(
            topic0(""),
            "0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470",
            "keccak256 of the empty string"
        );
    }

    #[test]
    fn the_two_exchange_versions_hash_differently() {
        assert_ne!(*TOPIC_ORDER_FILLED_V1, *TOPIC_ORDER_FILLED_V2);
        assert_ne!(*TOPIC_ORDERS_MATCHED_V1, *TOPIC_ORDERS_MATCHED_V2);
    }

    #[test]
    fn addresses_are_lower_case_and_well_formed() {
        let all = EXCHANGES
            .iter()
            .map(|e| e.address)
            .chain(COLLATERALS.iter().map(|c| c.address))
            .chain(WATCHED_COUNTERPARTIES.iter().copied())
            .chain([CONDITIONAL_TOKENS, NEG_RISK_ADAPTER]);

        for a in all {
            assert!(a.starts_with("0x"), "{a} is missing its prefix");
            assert_eq!(a.len(), 42, "{a} is not 20 bytes");
            assert_eq!(a, a.to_lowercase(), "{a} must be lower-case to compare");
        }
    }

    #[test]
    fn exchange_lookup_carries_version_and_neg_risk() {
        let e = exchange(CTF_EXCHANGE_V2).expect("v2 exchange is known");
        assert_eq!((e.version, e.neg_risk), (2, false));
        let n = exchange(NEG_RISK_CTF_EXCHANGE_V1).expect("neg-risk v1 is known");
        assert_eq!((n.version, n.neg_risk), (1, true));
        assert!(exchange("0xdead").is_none());
    }
}
