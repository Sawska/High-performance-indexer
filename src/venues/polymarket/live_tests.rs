//! Calls the real Polymarket endpoints and checks the responses still parse
//! into our types. Ignored by default: they need the network and their data
//! changes under them. Run with
//!
//!   cargo test live_ -- --ignored --test-threads=1

use std::time::Duration;

use tokio::sync::mpsc;

use super::clob::ClobApi;
use super::data::DataApi;
use super::gamma::GammaApi;
use super::types::{ActivityQuery, MarketEvent, PositionsQuery, PriceHistoryQuery, TradesQuery};
use super::user_pnl::UserPnlApi;
use super::websocket::PolymarketWebsocket;

struct Sample {
    condition_id: String,
    tokens: Vec<String>,
}

/// The most liquid live market on gamma's first page.
async fn sample() -> Sample {
    let page = GammaApi::new().list_markets(50, None).await.unwrap();
    assert!(!page.markets.is_empty(), "gamma returned no markets");
    let m = page
        .markets
        .iter()
        .filter(|m| m.enable_order_book == Some(true) && !m.closed.unwrap_or(false))
        .max_by(|a, b| a.liquidity_num.unwrap_or(0.0).total_cmp(&b.liquidity_num.unwrap_or(0.0)))
        .expect("no live order-book market on the first page");
    let tokens: Vec<String> = serde_json::from_str(m.clob_token_ids.as_deref().unwrap()).unwrap();
    assert_eq!(tokens.len(), 2, "binary market has two tokens");
    Sample {
        condition_id: m.condition_id.clone(),
        tokens,
    }
}

/// A wallet that traded the sample market.
async fn trader(s: &Sample) -> String {
    let query = TradesQuery {
        condition_id: Some(&s.condition_id),
        limit: 10,
        ..Default::default()
    };
    let page = DataApi::new().get_trades(query).await.unwrap();
    page.data.first().expect("no trades on the sample market").proxy_wallet.clone()
}

#[tokio::test]
#[ignore]
async fn live_gamma_markets_keyset_pages() {
    let gamma = GammaApi::new();
    let first = gamma.list_markets(5, None).await.unwrap();
    assert_eq!(first.markets.len(), 5);
    let cursor = first.next_cursor.expect("first page has a cursor");
    let second = gamma.list_markets(5, Some(&cursor)).await.unwrap();
    assert!(!second.markets.is_empty());
    assert_ne!(first.markets[0].id, second.markets[0].id, "the cursor advances");
}

#[tokio::test]
#[ignore]
async fn live_gamma_public_search_profiles() {
    // Parsing is the point; an empty result is fine.
    GammaApi::new().search_profiles("trader", 3).await.unwrap();
}

#[tokio::test]
#[ignore]
async fn live_clob_batch_endpoints() {
    let s = sample().await;
    let clob = ClobApi::new();
    let ids: Vec<&str> = s.tokens.iter().map(String::as_str).collect();

    let books = clob.get_books(&ids).await.unwrap();
    assert_eq!(books.len(), 2);
    assert!(books.iter().all(|b| b.timestamp.parse::<i64>().is_ok()), "timestamps are millis");

    let mids = clob.get_midpoints(&ids).await.unwrap();
    let spreads = clob.get_spreads(&ids).await.unwrap();
    for t in &s.tokens {
        assert!(mids.get(t).is_some_and(|v| v.parse::<f64>().is_ok()), "midpoint for {t}");
        assert!(spreads.get(t).is_some_and(|v| v.parse::<f64>().is_ok()), "spread for {t}");
    }

    let params: Vec<(&str, &str)> = ids.iter().flat_map(|t| [(*t, "BUY"), (*t, "SELL")]).collect();
    let prices = clob.get_prices(&params).await.unwrap();
    assert!(prices.values().all(|sides| sides.values().all(|v| v.parse::<f64>().is_ok())));

    let mut q = PriceHistoryQuery::new(&s.tokens[0]);
    q.interval = Some("1d");
    let history = clob.get_history(q).await.unwrap();
    assert!(history.history.iter().all(|h| (0.0..=1.0).contains(&h.p)));
}

#[tokio::test]
#[ignore]
async fn live_clob_single_token_endpoints() {
    let s = sample().await;
    let clob = ClobApi::new();
    let t = &s.tokens[0];
    clob.get_book(t).await.unwrap();
    clob.get_price(t, "BUY").await.unwrap();
    clob.get_midpoint(t).await.unwrap();
    clob.get_spread(t).await.unwrap();
    clob.get_last_trade_price(t).await.unwrap();
    let tick = clob.get_tick_size(t).await.unwrap();
    assert!(tick.minimum_tick_size > 0.0);
}

#[tokio::test]
#[ignore]
async fn live_data_market_endpoints() {
    let s = sample().await;
    let data = DataApi::new();

    let query = TradesQuery {
        condition_id: Some(&s.condition_id),
        limit: 20,
        ..Default::default()
    };
    let page = data.get_trades(query).await.unwrap();
    assert!(page.data.iter().all(|t| t.condition_id == s.condition_id));

    let holders = data.get_holders(&s.condition_id, 5).await.unwrap();
    assert!(holders.iter().all(|h| s.tokens.contains(&h.token)), "holders are keyed by token id");
}

#[tokio::test]
#[ignore]
async fn live_data_wallet_endpoints() {
    let s = sample().await;
    let wallet = trader(&s).await;
    let data = DataApi::new();

    data.get_positions(PositionsQuery { limit: 10, ..PositionsQuery::new(&wallet) }).await.unwrap();
    let activity = data.get_activity(ActivityQuery { limit: 10, ..ActivityQuery::new(&wallet) }).await.unwrap();
    assert!(!activity.is_empty(), "a wallet that traded has activity");
    let value = data.get_value(&wallet, None).await.unwrap();
    assert_eq!(value.len(), 1);
}

#[tokio::test]
#[ignore]
async fn live_user_pnl() {
    let s = sample().await;
    let wallet = trader(&s).await;
    UserPnlApi::new().get_pnl(&wallet, "all", "1d").await.unwrap();
}

#[tokio::test]
#[ignore]
async fn live_websocket_sends_a_book_on_subscribe() {
    let s = sample().await;
    let mut ws = PolymarketWebsocket::connect().await.unwrap();
    ws.subcribe(&s.tokens).await.unwrap();

    let (tx, mut rx) = mpsc::channel(16);
    let first = tokio::time::timeout(Duration::from_secs(20), async {
        tokio::select! {
            res = ws.run(tx) => panic!("socket ended early: {:?}", res.err().map(|e| e.to_string())),
            ev = rx.recv() => ev,
        }
    })
    .await
    .expect("no event within 20s");

    match first {
        Some(MarketEvent::Book(b)) => assert!(s.tokens.contains(&b.asset_id)),
        Some(MarketEvent::Unknow) | None => panic!("no decodable event"),
        Some(_) => {} // a trade or price change beat the snapshot; decoding is what matters
    }
}
