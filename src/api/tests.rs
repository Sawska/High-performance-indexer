//! End-to-end tests of the HTTP API over a real Postgres.
//!
//! Fixtures go in through the same writers the scraper uses, parsed from the
//! same JSON shapes the venue APIs return, and come back out through the
//! router exactly as the UI calls it. Each test gets its own fresh database
//! from `#[sqlx::test]`, created on the server `DATABASE_URL` points at; the
//! role needs CREATEDB:
//!
//!   DATABASE_URL=postgres://indexer:indexer@localhost:5432/indexer cargo test

use std::path::Path;
use std::time::Duration as StdDuration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

use crate::db::rest_writer::{self, Quote};
use crate::db::writer;
use crate::venues::polymarket::types::{
    Activity, History, ListMarkets, MarketEvent, MarketHolders, OrderBook, Position, Profile,
    Trade, UserPnlPoint, UserValue,
};

// ------------------------------------------------------------------ client

/// Drives the router in-process and keeps the session cookie like a browser.
struct Client {
    app: Router,
    cookie: Option<String>,
}

impl Client {
    async fn new(pool: &PgPool) -> Self {
        sqlx::raw_sql(include_str!("../../schema.sql"))
            .execute(pool)
            .await
            .unwrap();
        Self {
            app: super::router(pool.clone(), Path::new("/nonexistent")),
            cookie: None,
        }
    }

    async fn signed_in(pool: &PgPool) -> Self {
        let mut c = Self::new(pool).await;
        let (status, body) = c.post("/api/auth/register", EMAIL, PASSWORD).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        c
    }

    async fn request(&mut self, method: &str, path: &str, body: Option<Value>) -> (StatusCode, Value) {
        let mut req = Request::builder().method(method).uri(path);
        if let Some(c) = &self.cookie {
            req = req.header(header::COOKIE, c);
        }
        let req = match body {
            Some(b) => req
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(b.to_string())),
            None => req.body(Body::empty()),
        }
        .unwrap();

        let res = self.app.clone().oneshot(req).await.unwrap();
        let status = res.status();
        if let Some(set) = res.headers().get(header::SET_COOKIE) {
            let pair = set.to_str().unwrap().split(';').next().unwrap().to_string();
            // Removal answers with an empty value.
            self.cookie = (!pair.ends_with('=')).then_some(pair);
        }
        let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let json = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into()))
        };
        (status, json)
    }

    async fn post(&mut self, path: &str, email: &str, password: &str) -> (StatusCode, Value) {
        self.request("POST", path, Some(json!({ "email": email, "password": password })))
            .await
    }

    /// GET that must succeed.
    async fn get(&mut self, path: &str) -> Value {
        let (status, body) = self.request("GET", path, None).await;
        assert_eq!(status, StatusCode::OK, "GET {path}: {body}");
        body
    }
}

const EMAIL: &str = "tester@example.com";
const PASSWORD: &str = "correct horse";

fn ids(rows: &Value, key: &str) -> Vec<String> {
    rows.as_array()
        .unwrap()
        .iter()
        .map(|r| r[key].as_str().unwrap().to_string())
        .collect()
}

fn f(v: &Value) -> f64 {
    v.as_f64().unwrap_or_else(|| panic!("not a number: {v}"))
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

// ---------------------------------------------------------------- fixtures

const ALICE: &str = "0xaaa";
const BOB: &str = "0xbbb";
const CAROL: &str = "0xccc";

fn parse<T: serde::de::DeserializeOwned>(v: Value) -> T {
    serde_json::from_value(v).unwrap()
}

/// Two live markets (101 rain, 102 btc), one closed (103), three profiles,
/// and something in every table the API reads. Relative to `now`.
async fn seed(pool: &PgPool, now: DateTime<Utc>) {
    let secs = |d: Duration| (now - d).timestamp();
    let millis = |d: Duration| (now - d).timestamp_millis().to_string();

    let markets: Vec<ListMarkets> = parse(json!([
        {
            "id": "101", "question": "Will it rain in Paris tomorrow?", "conditionId": "0xc1",
            "slug": "rain-paris", "category": "Weather", "active": true, "closed": false,
            "archived": false, "acceptingOrders": true, "enableOrderBook": true,
            "startDate": "2026-01-01T00:00:00Z", "endDate": "2026-12-31T00:00:00Z",
            "outcomes": "[\"Yes\", \"No\"]", "outcomePrices": "[\"0.62\", \"0.38\"]",
            "clobTokenIds": "[\"tok-yes\", \"tok-no\"]",
            "volumeNum": 50000.0, "liquidityNum": 8000.0, "bestBid": 0.61, "bestAsk": 0.63,
            "lastTradePrice": 0.62, "spread": 0.02, "orderPriceMinTickSize": 0.01,
            "orderMinSize": 5.0, "icon": "https://img.example/rain.png",
            "image": "https://img.example/rain-big.png", "description": "Resolves YES if it rains.",
            "resolutionSource": "https://weather.example", "oneDayPriceChange": 0.05,
            "oneWeekPriceChange": -0.03, "oneMonthPriceChange": 0.1
        },
        {
            "id": "102", "question": "Will BTC close above 100k?", "conditionId": "0xc2",
            "slug": "btc-100k", "closed": false, "endDate": "2026-06-30T00:00:00Z",
            "outcomes": "[\"Yes\", \"No\"]", "outcomePrices": "[\"0.3\", \"0.7\"]",
            "clobTokenIds": "[\"tok2-yes\", \"tok2-no\"]",
            "volumeNum": 90000.0, "liquidityNum": 1000.0, "spread": 0.05,
            "oneDayPriceChange": -0.10
        },
        {
            "id": "103", "question": "Old closed market", "conditionId": "0xc3", "closed": true,
            "outcomes": "[\"A\", \"B\"]", "clobTokenIds": "[\"tok3-a\", \"tok3-b\"]",
            "volumeNum": 1000000.0, "liquidityNum": 0.0
        }
    ]));
    rest_writer::upsert_markets(pool, &markets, now).await.unwrap();

    let profiles: Vec<Profile> = parse(json!([
        { "proxyWallet": ALICE, "name": "alice", "bio": "I trade weather", "verified": true },
        { "proxyWallet": BOB, "pseudonym": "Brave-Bee" },
        { "proxyWallet": CAROL }
    ]));
    rest_writer::upsert_profiles(pool, &profiles, now).await.unwrap();

    // tok-yes has an older snapshot that must be ignored, and a newer one
    // twelve levels deep that the API must cut to ten a side.
    let level = |p: f64| json!({ "price": format!("{p:.2}"), "size": "100" });
    let books: Vec<OrderBook> = parse(json!([
        {
            "market": "0xc1", "asset_id": "tok-yes", "timestamp": millis(Duration::minutes(10)),
            "hash": "old", "bids": [level(0.40)], "asks": [level(0.90)],
            "tick_size": "0.1", "min_order_size": "1", "neg_risk": false
        },
        {
            "market": "0xc1", "asset_id": "tok-yes", "timestamp": millis(Duration::minutes(1)),
            "hash": "new",
            "bids": (0..12).map(|i| level(0.61 - i as f64 * 0.01)).collect::<Vec<_>>(),
            "asks": (0..12).map(|i| level(0.63 + i as f64 * 0.01)).collect::<Vec<_>>(),
            "tick_size": "0.01", "min_order_size": "5", "neg_risk": true,
            "last_trade_price": "0.62"
        },
        {
            "market": "0xc1", "asset_id": "tok-no", "timestamp": millis(Duration::minutes(1)),
            "hash": "new-no", "bids": [level(0.37), level(0.36)], "asks": [level(0.39), level(0.40)],
            "tick_size": "0.01", "min_order_size": "5", "neg_risk": true
        }
    ]));
    rest_writer::insert_books(pool, &books, now).await.unwrap();

    let q = |asset: &str, kind: &str, side: Option<&str>, value: f64| Quote {
        asset_id: asset.into(),
        kind: kind.into(),
        side: side.map(Into::into),
        value,
    };
    // Hours apart, so each lands in its own hourly spread bucket.
    rest_writer::insert_quotes(pool, &[q("tok-yes", "spread", None, 0.9)], now - Duration::days(40))
        .await
        .unwrap();
    rest_writer::insert_quotes(
        pool,
        &[
            q("tok-yes", "midpoint", None, 0.55),
            q("tok-yes", "spread", None, 0.10),
            q("tok-yes", "price", Some("BUY"), 0.50),
            q("tok-yes", "price", Some("SELL"), 0.60),
        ],
        now - Duration::hours(3),
    )
    .await
    .unwrap();
    rest_writer::insert_quotes(
        pool,
        &[
            q("tok-yes", "midpoint", None, 0.62),
            q("tok-yes", "spread", None, 0.02),
            q("tok-yes", "price", Some("BUY"), 0.61),
            q("tok-yes", "price", Some("SELL"), 0.63),
            q("tok-no", "midpoint", None, 0.38),
            q("tok-no", "spread", None, 0.02),
        ],
        now - Duration::minutes(1),
    )
    .await
    .unwrap();

    let h = |d: Duration, p: f64| History { t: secs(d) as u32, p };
    rest_writer::insert_price_history(
        pool,
        "tok-yes",
        &[h(Duration::days(40), 0.1), h(Duration::days(2), 0.5), h(Duration::days(1), 0.6)],
    )
    .await
    .unwrap();

    let trade = |wallet: &str, cond: &str, token: &str, side: &str, size: f64, price: f64, ago: Duration, tx: &str| {
        json!({
            "proxy_wallet": wallet, "condition_id": cond, "token_id": token, "side": side,
            "size": size, "price": price, "timestamp": secs(ago), "transaction_hash": tx,
            "outcome": "Yes", "title": format!("title of {cond}")
        })
    };
    let trades: Vec<Trade> = parse(json!([
        trade(ALICE, "0xc1", "tok-yes", "BUY", 100.0, 0.6, Duration::hours(1), "0xt1"),
        trade(BOB, "0xc1", "tok-no", "SELL", 50.0, 0.4, Duration::hours(2), "0xt2"),
        trade(ALICE, "0xc2", "tok2-yes", "BUY", 2000.0, 0.5, Duration::hours(3), "0xt3"),
        trade(BOB, "0xc1", "tok-yes", "BUY", 10.0, 0.5, Duration::hours(30), "0xt4")
    ]));
    rest_writer::insert_data_trades(pool, &trades, now).await.unwrap();

    let holder = |wallet: &str, amount: f64| json!({ "proxyWallet": wallet, "asset": "tok-yes", "amount": amount });
    let stale: Vec<MarketHolders> = parse(json!([{ "token": "tok-yes", "holders": [holder(CAROL, 999_999.0)] }]));
    rest_writer::insert_holders(pool, &stale, now - Duration::days(1)).await.unwrap();
    let mut current = vec![holder(ALICE, 5000.0), holder(BOB, 4000.0)];
    current.extend((1..=10).map(|i| holder(&format!("0xw{i:02}"), 100.0 * i as f64)));
    let fresh: Vec<MarketHolders> = parse(json!([{ "token": "tok-yes", "holders": current }]));
    rest_writer::insert_holders(pool, &fresh, now).await.unwrap();

    let position = |asset: &str, cond: &str, current: f64, redeemable: bool| {
        json!({
            "proxyWallet": ALICE, "asset": asset, "conditionId": cond, "size": 100.0,
            "avgPrice": 0.5, "initialValue": 50.0, "currentValue": current, "cashPnl": current - 50.0,
            "percentPnl": (current - 50.0) * 2.0, "totalBought": 50.0, "realizedPnl": 3.0,
            "percentRealizedPnl": 6.0, "curPrice": current / 100.0, "redeemable": redeemable,
            "mergeable": false, "outcome": "Yes", "endDate": "2026-12-31"
        })
    };
    let old: Vec<Position> = parse(json!([position("tok3-a", "0xc3", 10.0, false)]));
    rest_writer::insert_positions(pool, &old, now - Duration::days(1)).await.unwrap();
    let book: Vec<Position> = parse(json!([
        position("tok-gone", "0xunknown", 0.0, true),
        position("tok-yes", "0xc1", 62.0, false)
    ]));
    rest_writer::insert_positions(pool, &book, now).await.unwrap();

    let activity = |wallet: &str, kind: &str, side: Option<&str>, cond: &str, ago: Duration, tx: &str| {
        json!({
            "proxyWallet": wallet, "timestamp": secs(ago), "conditionId": cond, "type": kind,
            "size": 10.0, "usdcSize": 5.0, "transactionHash": tx, "price": 0.5, "asset": "a",
            "side": side, "title": if cond == "0xunknown" { "Some resolved market" } else { "t" }
        })
    };
    let acts: Vec<Activity> = parse(json!([
        activity(ALICE, "TRADE", Some("BUY"), "0xc1", Duration::hours(1), "0xa1"),
        activity(ALICE, "REDEEM", None, "0xunknown", Duration::hours(2), "0xa2"),
        activity(BOB, "SPLIT", None, "0xc2", Duration::hours(3), "0xa3"),
        activity(BOB, "TRADE", Some("SELL"), "0xc1", Duration::hours(4), "0xa4")
    ]));
    rest_writer::insert_activity(pool, &acts, now).await.unwrap();

    let value = |v: f64| UserValue { user: ALICE.into(), value: v };
    rest_writer::insert_user_values(pool, &[value(100.0)], None, now - Duration::days(1)).await.unwrap();
    rest_writer::insert_user_values(pool, &[value(150.0)], None, now).await.unwrap();
    // A per-market value is not the portfolio and must not be read as one.
    rest_writer::insert_user_values(pool, &[value(999.0)], Some("0xc1"), now + Duration::seconds(1))
        .await
        .unwrap();

    let p = |d: Duration, v: f64| UserPnlPoint { t: secs(d), p: v };
    rest_writer::insert_user_pnl(
        pool,
        ALICE,
        &[p(Duration::days(2), -5.0), p(Duration::days(1), 10.0), p(Duration::zero(), 20.0)],
    )
    .await
    .unwrap();

    seed_websocket(pool, &millis(Duration::seconds(30))).await;
}

/// Pushes market-channel events through the websocket writer and waits for
/// its batch to land.
async fn seed_websocket(pool: &PgPool, ts: &str) {
    let events: Vec<MarketEvent> = parse(json!([
        { "event_type": "last_trade_price", "market": "0xc1", "asset_id": "tok-yes", "price": "0.62",
          "size": "25", "side": "BUY", "timestamp": ts, "transaction_hash": "0xws1" },
        { "event_type": "price_change", "market": "0xc1", "timestamp": ts, "price_changes": [
            { "asset_id": "tok-yes", "price": "0.61", "size": "300", "side": "BUY", "hash": "h1", "best_ask": "0.63" },
            { "asset_id": "tok-no", "price": "0.39", "size": "0", "side": "SELL", "hash": "h2", "best_ask": "0.39" }
        ]},
        { "event_type": "tick_size_change", "market": "0xc1", "asset_id": "tok-yes",
          "old_tick_size": "0.1", "new_tick_size": "0.01", "timestamp": ts }
    ]));
    let tx = writer::spawn(pool.clone());
    for ev in events {
        tx.send(ev).await.unwrap();
    }
    drop(tx);

    for _ in 0..100 {
        let n: i64 = sqlx::query_scalar(
            "SELECT (SELECT count(*) FROM trades) + (SELECT count(*) FROM price_changes)
                  + (SELECT count(*) FROM tick_size_changes)",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        if n == 4 {
            return;
        }
        tokio::time::sleep(StdDuration::from_millis(50)).await;
    }
    panic!("websocket writer did not flush");
}

async fn seeded(pool: PgPool) -> Client {
    let c = Client::signed_in(&pool).await;
    seed(&pool, Utc::now()).await;
    c
}

// -------------------------------------------------------------------- auth

#[sqlx::test(migrations = false)]
async fn schema_applies_twice(pool: PgPool) {
    Client::new(&pool).await;
    Client::new(&pool).await;
}

#[sqlx::test(migrations = false)]
async fn auth_register_login_logout(pool: PgPool) {
    let mut c = Client::new(&pool).await;

    let (status, body) = c.request("GET", "/api/auth/me", None).await;
    assert_eq!((status, &body["error"]), (StatusCode::UNAUTHORIZED, &json!("not signed in")));

    let (status, _) = c.post("/api/auth/register", "not-an-email", PASSWORD).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, body) = c.post("/api/auth/register", EMAIL, "short").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("password"));
    assert!(c.cookie.is_none());

    let (status, body) = c.post("/api/auth/register", "  Tester@Example.com ", PASSWORD).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["email"], EMAIL);
    assert!(c.cookie.as_deref().unwrap().starts_with("session="));
    assert_eq!(c.get("/api/auth/me").await["email"], EMAIL);

    let stored: String = sqlx::query_scalar("SELECT token_hash FROM sessions").fetch_one(&pool).await.unwrap();
    let token = c.cookie.clone().unwrap();
    assert!(!token.contains(&stored), "the database holds only the token's hash");

    let (status, _) = c.post("/api/auth/register", "TESTER@example.com", PASSWORD).await;
    assert_eq!(status, StatusCode::CONFLICT);

    let (status, _) = c.request("POST", "/api/auth/logout", Some(json!({}))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(c.cookie.is_none(), "logout clears the cookie");
    let (status, _) = c.request("GET", "/api/auth/me", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // The old token is dead server-side too, not just forgotten by the browser.
    c.cookie = Some(token);
    let (status, _) = c.request("GET", "/api/overview", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    c.cookie = None;

    let (status, wrong_pw) = c.post("/api/auth/login", EMAIL, "wrong password").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, no_user) = c.post("/api/auth/login", "nobody@example.com", PASSWORD).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(wrong_pw, no_user, "a miss must not reveal whether the email exists");

    let (status, body) = c.post("/api/auth/login", " TESTER@example.com", PASSWORD).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(c.get("/api/auth/me").await["email"], EMAIL);
}

#[sqlx::test(migrations = false)]
async fn every_data_route_requires_a_session(pool: PgPool) {
    let mut c = Client::new(&pool).await;
    for path in [
        "/api/status",
        "/api/overview",
        "/api/markets",
        "/api/markets/101",
        "/api/traders",
        "/api/traders/0xaaa",
        "/api/trades",
        "/api/activity",
    ] {
        let (status, body) = c.request("GET", path, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{path}");
        assert_eq!(body["error"], "not signed in", "{path}");
    }
}

#[sqlx::test(migrations = false)]
async fn unknown_api_path_is_a_json_404(pool: PgPool) {
    let mut c = Client::signed_in(&pool).await;
    let (status, body) = c.request("GET", "/api/nope", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "no such endpoint");
}

#[sqlx::test(migrations = false)]
async fn expired_sessions_are_rejected(pool: PgPool) {
    let mut c = Client::signed_in(&pool).await;
    sqlx::query("UPDATE sessions SET expires_at = now() - interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();
    let (status, _) = c.request("GET", "/api/overview", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ------------------------------------------------------------------ status

#[sqlx::test(migrations = false)]
async fn status_lists_every_table(pool: PgPool) {
    let mut c = Client::signed_in(&pool).await;
    let body = c.get("/api/status").await;
    let tables = ids(&body["tables"], "name");
    for t in ["markets", "quotes", "book_levels", "data_trades", "activity", "trades", "users"] {
        assert!(tables.contains(&t.to_string()), "{t} missing from {tables:?}");
    }
    assert!(body["live"]["cycle"].is_u64());
    assert!(body["live"]["stages"].is_array());
    assert!(body["now"].is_string());
}

// ---------------------------------------------------------------- overview

#[sqlx::test(migrations = false)]
async fn overview_totals_and_lists(pool: PgPool) {
    let mut c = seeded(pool).await;
    let body = c.get("/api/overview").await;

    let t = &body["totals"];
    assert_eq!(t["markets"], 3);
    assert_eq!(t["live_markets"], 2);
    assert!(close(f(&t["live_volume"]), 140_000.0));
    assert!(close(f(&t["live_liquidity"]), 9_000.0));
    assert_eq!(t["wallets"], 3);
    assert_eq!(t["trades_24h"], 3, "the 30h-old trade is outside the window");
    assert!(close(f(&t["volume_24h"]), 60.0 + 20.0 + 1000.0));

    assert_eq!(ids(&body["top_markets"], "id"), ["102", "101"], "live only, by volume");
    assert_eq!(ids(&body["recent_trades"], "transaction_hash"), ["0xt1", "0xt2", "0xt3", "0xt4"]);
    assert_eq!(body["recent_trades"][0]["name"], "alice", "trades join the profile");
}

// ----------------------------------------------------------------- markets

#[sqlx::test(migrations = false)]
async fn markets_filter_sort_search_and_page(pool: PgPool) {
    let mut c = seeded(pool).await;

    let page = c.get("/api/markets").await;
    assert_eq!(page["total"], 2);
    assert_eq!(ids(&page["rows"], "id"), ["102", "101"]);

    let m = &page["rows"][1];
    assert_eq!(m["outcomes"], json!(["Yes", "No"]));
    assert_eq!(m["outcome_prices"], json!([0.62, 0.38]));
    assert_eq!(m["token_ids"], json!(["tok-yes", "tok-no"]));
    assert_eq!(m["icon"], "https://img.example/rain.png");
    assert!(close(f(&m["one_day_change"]), 0.05));
    assert_eq!(m["closed"], false);

    let cases = [
        ("?status=closed", vec!["103"]),
        ("?status=all", vec!["103", "102", "101"]),
        ("?sort=liquidity", vec!["101", "102"]),
        ("?sort=ending", vec!["102", "101"]),
        ("?sort=spread", vec!["102", "101"]),
        ("?sort=movers", vec!["102", "101"]),
        ("?sort=bogus", vec!["102", "101"]),
        ("?q=PARIS", vec!["101"]),
        ("?q=btc-100k", vec!["102"]),
        ("?q=nothing-matches", vec![]),
        ("?limit=1&offset=1", vec!["101"]),
    ];
    for (qs, want) in cases {
        let page = c.get(&format!("/api/markets{qs}")).await;
        assert_eq!(ids(&page["rows"], "id"), want, "{qs}");
    }
    assert_eq!(c.get("/api/markets?limit=1").await["total"], 2, "total ignores paging");

    // Search text is bound, never spliced into the SQL.
    let page = c.get("/api/markets?q=%27%3B%20DROP%20TABLE%20markets%3B%20--").await;
    assert_eq!(page["total"], 0);
    assert_eq!(c.get("/api/markets?status=all").await["total"], 3);
}

#[sqlx::test(migrations = false)]
async fn market_detail_returns_every_section(pool: PgPool) {
    let mut c = seeded(pool).await;

    let by_condition = c.get("/api/markets/0xc1").await;
    let d = c.get("/api/markets/101").await;
    assert_eq!(by_condition["market"]["id"], "101", "the condition id also resolves");
    assert_eq!(d["market"]["question"], "Will it rain in Paris tomorrow?");

    let (status, body) = c.request("GET", "/api/markets/999", None).await;
    assert_eq!((status, &body["error"]), (StatusCode::NOT_FOUND, &json!("market not found")));

    let info = &d["info"];
    assert_eq!(info["description"], "Resolves YES if it rains.");
    assert_eq!(info["image"], "https://img.example/rain-big.png");
    assert_eq!(info["resolution_source"], "https://weather.example");
    assert_eq!(info["start_date"], "2026-01-01T00:00:00Z");
    assert_eq!((&info["active"], &info["accepting_orders"]), (&json!(true), &json!(true)));
    assert!(close(f(&info["tick_size"]), 0.01));
    assert!(close(f(&info["min_order_size"]), 5.0));
    assert!(close(f(&info["one_week_change"]), -0.03));
    assert!(close(f(&info["one_month_change"]), 0.1));

    let history: Vec<f64> = d["history"].as_array().unwrap().iter().map(|p| f(&p["price"])).collect();
    assert_eq!(history, [0.5, 0.6], "30-day window, oldest first");

    let spread_of = |token: &str| -> Vec<f64> {
        d["spread_history"].as_array().unwrap().iter()
            .filter(|p| p["token"] == token).map(|p| f(&p["price"])).collect()
    };
    assert_eq!(spread_of("tok-yes"), [0.10, 0.02], "hourly buckets, 40-day-old one dropped");
    assert_eq!(spread_of("tok-no"), [0.02]);

    // Only the newest value per (token, kind, side).
    let quote = |token: &str, kind: &str, side: Option<&str>| -> Vec<f64> {
        d["quotes"].as_array().unwrap().iter()
            .filter(|q| q["asset_id"] == token && q["kind"] == kind && q["side"] == json!(side))
            .map(|q| f(&q["value"])).collect()
    };
    assert_eq!(quote("tok-yes", "midpoint", None), [0.62]);
    assert_eq!(quote("tok-yes", "spread", None), [0.02]);
    assert_eq!(quote("tok-yes", "price", Some("BUY")), [0.61]);
    assert_eq!(quote("tok-yes", "price", Some("SELL")), [0.63]);
    assert_eq!(quote("tok-no", "midpoint", None), [0.38]);
    assert_eq!(d["quotes"].as_array().unwrap().len(), 6);

    let books = d["books"].as_array().unwrap();
    assert_eq!(books.len(), 2);
    let yes = books.iter().find(|b| b["asset_id"] == "tok-yes").unwrap();
    assert_eq!(yes["neg_risk"], true, "the newest snapshot's header");
    assert!(close(f(&yes["tick_size"]), 0.01));
    assert!(close(f(&yes["min_order_size"]), 5.0));
    assert!(close(f(&yes["last_trade_price"]), 0.62));

    let levels = |token: &str, side: &str| -> Vec<f64> {
        d["book"].as_array().unwrap().iter()
            .filter(|l| l["asset_id"] == token && l["side"] == side)
            .map(|l| f(&l["price"])).collect()
    };
    let bids = levels("tok-yes", "bid");
    let asks = levels("tok-yes", "ask");
    assert_eq!((bids.len(), asks.len()), (10, 10), "cut to ten a side");
    assert!(close(bids[0], 0.61) && bids.windows(2).all(|w| w[0] > w[1]), "best bid first: {bids:?}");
    assert!(close(asks[0], 0.63) && asks.windows(2).all(|w| w[0] < w[1]), "best ask first: {asks:?}");
    assert!(!bids.iter().any(|p| close(*p, 0.40)), "older snapshot ignored");
    assert_eq!((levels("tok-no", "bid").len(), levels("tok-no", "ask").len()), (2, 2));

    let holders = d["holders"].as_array().unwrap();
    assert_eq!(holders.len(), 10, "top ten per token");
    assert_eq!(holders[0]["proxy_wallet"], ALICE);
    assert_eq!(holders[0]["name"], "alice");
    assert_eq!(holders[1]["pseudonym"], "Brave-Bee");
    assert!(holders.windows(2).all(|w| f(&w[0]["amount"]) >= f(&w[1]["amount"])));
    assert!(!holders.iter().any(|h| h["proxy_wallet"] == CAROL), "stale capture ignored");

    assert_eq!(ids(&d["trades"], "transaction_hash"), ["0xt1", "0xt2", "0xt4"], "this market only");

    let ws = &d["ws_trades"][0];
    assert_eq!(d["ws_trades"].as_array().unwrap().len(), 1);
    assert_eq!((&ws["asset_id"], &ws["side"], &ws["transaction_hash"]), (&json!("tok-yes"), &json!("BUY"), &json!("0xws1")));
    assert!(close(f(&ws["price"]), 0.62) && close(f(&ws["size"]), 25.0));
    assert_eq!(d["price_changes"].as_array().unwrap().len(), 2);
    assert_eq!(d["tick_changes"].as_array().unwrap().len(), 1);
    assert!(close(f(&d["tick_changes"][0]["new_tick_size"]), 0.01));
    assert!(close(f(&d["tick_changes"][0]["old_tick_size"]), 0.1));
}

#[sqlx::test(migrations = false)]
async fn market_detail_is_empty_not_broken_without_scrape_data(pool: PgPool) {
    let mut c = Client::signed_in(&pool).await;
    let markets: Vec<ListMarkets> = parse(json!([{ "id": "7", "conditionId": "0x7" }]));
    rest_writer::upsert_markets(&pool, &markets, Utc::now()).await.unwrap();

    let d = c.get("/api/markets/7").await;
    assert_eq!(d["market"]["token_ids"], json!([]));
    assert_eq!(d["market"]["outcomes"], json!([]));
    for key in ["history", "spread_history", "quotes", "books", "book", "holders", "trades", "ws_trades", "price_changes", "tick_changes"] {
        assert_eq!(d[key], json!([]), "{key}");
    }
    assert!(d["info"]["description"].is_null());
}

// ----------------------------------------------------------------- traders

#[sqlx::test(migrations = false)]
async fn traders_sort_and_search(pool: PgPool) {
    let mut c = seeded(pool).await;

    let page = c.get("/api/traders").await;
    assert_eq!(page["total"], 3);
    let alice = &page["rows"][0];
    assert_eq!(alice["proxy_wallet"], ALICE);
    assert!(close(f(&alice["pnl"]), 20.0), "latest pnl point");
    assert!(close(f(&alice["value"]), 150.0), "portfolio value, not a per-market one");
    assert!(close(f(&alice["volume"]), 60.0 + 1000.0));
    assert_eq!(alice["trades"], 2);

    let cases = [
        ("?sort=volume", vec![ALICE, BOB, CAROL]),
        ("?sort=value", vec![ALICE, BOB, CAROL]),
        ("?q=ALICE", vec![ALICE]),
        ("?q=brave", vec![BOB]),
        ("?q=0xccc", vec![CAROL]),
        ("?limit=1&offset=2", vec![CAROL]),
    ];
    for (qs, want) in cases {
        let page = c.get(&format!("/api/traders{qs}")).await;
        assert_eq!(ids(&page["rows"], "proxy_wallet"), want, "{qs}");
    }
    let bob = &c.get("/api/traders?q=brave").await["rows"][0];
    assert!(bob["pnl"].is_null() && bob["value"].is_null());
    assert!(close(f(&bob["volume"]), 20.0 + 5.0));
}

#[sqlx::test(migrations = false)]
async fn trader_detail_returns_every_section(pool: PgPool) {
    let mut c = seeded(pool).await;
    let d = c.get(&format!("/api/traders/{ALICE}")).await;

    assert_eq!(d["trader"]["name"], "alice");
    assert_eq!(d["profile"]["bio"], "I trade weather");
    assert_eq!(d["profile"]["verified"], true);

    let pnl: Vec<f64> = d["pnl"].as_array().unwrap().iter().map(|p| f(&p["pnl"])).collect();
    assert_eq!(pnl, [-5.0, 10.0, 20.0], "oldest first");
    let values: Vec<f64> = d["values"].as_array().unwrap().iter().map(|p| f(&p["value"])).collect();
    assert_eq!(values, [100.0, 150.0], "per-market value rows excluded");

    let positions = d["positions"].as_array().unwrap();
    assert_eq!(ids(&d["positions"], "asset"), ["tok-yes", "tok-gone"], "newest capture, by value");
    let open = &positions[0];
    assert_eq!(open["has_market"], true);
    assert_eq!(open["question"], "Will it rain in Paris tomorrow?");
    assert!(close(f(&open["initial_value"]), 50.0));
    assert!(close(f(&open["current_value"]), 62.0));
    assert!(close(f(&open["total_bought"]), 50.0));
    assert!(close(f(&open["realized_pnl"]), 3.0));
    assert!(close(f(&open["cash_pnl"]), 12.0));
    assert_eq!((&open["redeemable"], &open["mergeable"]), (&json!(false), &json!(false)));
    assert_eq!(open["end_date"], "2026-12-31");
    let gone = &positions[1];
    assert_eq!(gone["has_market"], false);
    assert_eq!(gone["question"], "Some resolved market", "title falls back to activity");
    assert_eq!(gone["redeemable"], true);

    assert_eq!(ids(&d["activity"], "transaction_hash"), ["0xa1", "0xa2"]);
    assert_eq!(d["activity"][1]["activity_type"], "REDEEM");

    let (status, body) = c.request("GET", "/api/traders/0xnobody", None).await;
    assert_eq!((status, &body["error"]), (StatusCode::NOT_FOUND, &json!("trader not found")));
}

// ------------------------------------------------------ trades and activity

#[sqlx::test(migrations = false)]
async fn trades_filters_and_paging(pool: PgPool) {
    let mut c = seeded(pool).await;

    let cases = [
        ("", vec!["0xt1", "0xt2", "0xt3", "0xt4"]),
        ("?side=buy", vec!["0xt1", "0xt3", "0xt4"]),
        ("?side=SELL", vec!["0xt2"]),
        ("?min_usd=50", vec!["0xt1", "0xt3"]),
        ("?market=0xc1", vec!["0xt1", "0xt2", "0xt4"]),
        ("?wallet=0xbbb", vec!["0xt2", "0xt4"]),
        ("?market=0xc1&wallet=0xaaa", vec!["0xt1"]),
        ("?market=0xc2&side=SELL", vec![]),
        ("?limit=2", vec!["0xt1", "0xt2"]),
        ("?limit=2&offset=2", vec!["0xt3", "0xt4"]),
        ("?offset=-3", vec!["0xt1", "0xt2", "0xt3", "0xt4"]),
    ];
    for (qs, want) in cases {
        let rows = c.get(&format!("/api/trades{qs}")).await;
        assert_eq!(ids(&rows, "transaction_hash"), want, "{qs}");
    }

    let t = &c.get("/api/trades?limit=1").await[0];
    assert_eq!((&t["proxy_wallet"], &t["name"], &t["side"]), (&json!(ALICE), &json!("alice"), &json!("BUY")));
    assert!(close(f(&t["size"]), 100.0) && close(f(&t["price"]), 0.6));
    assert_eq!(t["condition_id"], "0xc1");
}

#[sqlx::test(migrations = false)]
async fn activity_feed_filters_and_paging(pool: PgPool) {
    let mut c = seeded(pool).await;

    let cases = [
        ("", vec!["0xa1", "0xa2", "0xa3", "0xa4"]),
        ("?type=redeem", vec!["0xa2"]),
        ("?type=TRADE", vec!["0xa1", "0xa4"]),
        ("?wallet=0xbbb", vec!["0xa3", "0xa4"]),
        ("?type=SPLIT&wallet=0xbbb", vec!["0xa3"]),
        ("?type=SPLIT&wallet=0xaaa", vec![]),
        ("?limit=1&offset=1", vec!["0xa2"]),
    ];
    for (qs, want) in cases {
        let rows = c.get(&format!("/api/activity{qs}")).await;
        assert_eq!(ids(&rows, "transaction_hash"), want, "{qs}");
    }

    let a = &c.get("/api/activity?limit=1").await[0];
    assert_eq!((&a["proxy_wallet"], &a["name"]), (&json!(ALICE), &json!("alice")));
    assert_eq!((&a["activity_type"], &a["side"]), (&json!("TRADE"), &json!("BUY")));
    assert!(close(f(&a["usdc_size"]), 5.0));
    let b = &c.get("/api/activity?wallet=0xbbb&limit=1").await[0];
    assert_eq!(b["pseudonym"], "Brave-Bee");
}
