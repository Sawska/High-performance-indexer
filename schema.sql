CREATE TABLE IF NOT EXISTS trades (
    asset_id         TEXT        NOT NULL,
    market           TEXT        NOT NULL,
    ts               TIMESTAMPTZ NOT NULL,
    price            NUMERIC     NOT NULL,
    size             NUMERIC,
    side             TEXT        NOT NULL,
    fee_rate_bps     NUMERIC,
    transaction_hash TEXT
);
CREATE INDEX IF NOT EXISTS trades_asset_ts ON trades (asset_id, ts DESC);

CREATE TABLE IF NOT EXISTS price_changes (
    asset_id TEXT        NOT NULL,
    market   TEXT        NOT NULL,
    ts       TIMESTAMPTZ NOT NULL,
    price    NUMERIC     NOT NULL,
    size     NUMERIC     NOT NULL,
    side     TEXT        NOT NULL,
    best_ask NUMERIC,
    hash     TEXT
);
CREATE INDEX IF NOT EXISTS price_changes_asset_ts ON price_changes (asset_id, ts DESC);

CREATE TABLE IF NOT EXISTS tick_size_changes (
    asset_id      TEXT        NOT NULL,
    market        TEXT        NOT NULL,
    ts            TIMESTAMPTZ NOT NULL,
    old_tick_size NUMERIC,
    new_tick_size NUMERIC     NOT NULL
);
CREATE INDEX IF NOT EXISTS tick_size_changes_asset_ts
    ON tick_size_changes (asset_id, ts DESC);

-- ---------------------------------------------------------------------------
-- REST-scraped data.
--
-- The websocket tables above are event streams: append-only, never revised.
-- These are a mix of three shapes, and the shape decides the key:
--   dimension  (markets, profiles)        -- current state, upserted in place
--   snapshot   (books, positions, holders, user_values, quotes)
--                                         -- API returns "now", so captured_at
--                                            is part of the key and history
--                                            accumulates row by row
--   event      (data_trades, activity, price_history, user_pnl)
--                                         -- immutable, keyed naturally so a
--                                            re-scrape of overlapping pages is
--                                            a no-op
-- ---------------------------------------------------------------------------

-- Gamma /markets/keyset. Explicit columns cover what is queried; `raw` keeps
-- the full payload so a new upstream field is not lost before the schema
-- catches up.
CREATE TABLE IF NOT EXISTS markets (
    id                        TEXT PRIMARY KEY,
    question                  TEXT,
    condition_id              TEXT        NOT NULL,
    slug                      TEXT,
    category                  TEXT,
    active                    BOOLEAN,
    closed                    BOOLEAN,
    archived                  BOOLEAN,
    accepting_orders          BOOLEAN,
    enable_order_book         BOOLEAN,
    start_date                TEXT,
    end_date                  TEXT,
    outcomes                  TEXT,
    outcome_prices            TEXT,
    clob_token_ids            TEXT,
    volume_num                NUMERIC,
    liquidity_num             NUMERIC,
    best_bid                  NUMERIC,
    best_ask                  NUMERIC,
    last_trade_price          NUMERIC,
    spread                    NUMERIC,
    order_price_min_tick_size NUMERIC,
    order_min_size            NUMERIC,
    raw                       JSONB       NOT NULL,
    scraped_at                TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS markets_condition_id ON markets (condition_id);
CREATE INDEX IF NOT EXISTS markets_slug         ON markets (slug);

-- Gamma /public-search, plus the profile fields embedded in holders/trades.
CREATE TABLE IF NOT EXISTS profiles (
    proxy_wallet            TEXT PRIMARY KEY,
    name                    TEXT,
    pseudonym               TEXT,
    bio                     TEXT,
    profile_image           TEXT,
    profile_image_optimized TEXT,
    verified                BOOLEAN,
    display_username_public BOOLEAN,
    scraped_at              TIMESTAMPTZ NOT NULL
);

-- CLOB /book and /books. Header and levels split so one row per level keeps
-- depth queryable; the pair shares (asset_id, ts, hash).
CREATE TABLE IF NOT EXISTS book_snapshots (
    asset_id         TEXT        NOT NULL,
    market           TEXT        NOT NULL,
    ts               TIMESTAMPTZ NOT NULL,
    hash             TEXT        NOT NULL,
    min_order_size   NUMERIC,
    tick_size        NUMERIC,
    neg_risk         BOOLEAN,
    last_trade_price NUMERIC,
    scraped_at       TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (asset_id, ts, hash)
);

CREATE TABLE IF NOT EXISTS book_levels (
    asset_id  TEXT        NOT NULL,
    ts        TIMESTAMPTZ NOT NULL,
    hash      TEXT        NOT NULL,
    side      TEXT        NOT NULL,   -- 'bid' | 'ask'
    level_idx INT         NOT NULL,   -- 0 = best, as returned by the API
    price     NUMERIC     NOT NULL,
    size      NUMERIC     NOT NULL,
    PRIMARY KEY (asset_id, ts, hash, side, level_idx)
);

-- CLOB /price, /midpoint, /spread, /last-trade-price, /tick-size. One narrow
-- table instead of five: every endpoint returns a single scalar per token, and
-- `kind` keeps them apart.
CREATE TABLE IF NOT EXISTS quotes (
    asset_id   TEXT        NOT NULL,
    kind       TEXT        NOT NULL,   -- 'price'|'midpoint'|'spread'|'last_trade_price'|'tick_size'
    side       TEXT,                   -- only set for 'price' and 'last_trade_price'
    value      NUMERIC     NOT NULL,
    captured_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS quotes_asset_kind_ts
    ON quotes (asset_id, kind, captured_at DESC);

-- CLOB /prices-history.
CREATE TABLE IF NOT EXISTS price_history (
    market TEXT        NOT NULL,
    ts     TIMESTAMPTZ NOT NULL,
    price  NUMERIC     NOT NULL,
    PRIMARY KEY (market, ts)
);

-- Data API /v2/trades. Separate from the websocket `trades` table: this one is
-- wallet-attributed and arrives by backfill, not live.
CREATE TABLE IF NOT EXISTS data_trades (
    transaction_hash TEXT        NOT NULL,
    proxy_wallet     TEXT        NOT NULL,
    condition_id     TEXT        NOT NULL,
    token_id         TEXT        NOT NULL,
    side             TEXT        NOT NULL,
    size             NUMERIC     NOT NULL,
    price            NUMERIC     NOT NULL,
    ts               TIMESTAMPTZ NOT NULL,
    outcome          TEXT,
    outcome_index    BIGINT,
    title            TEXT,
    slug             TEXT,
    event_slug       TEXT,
    scraped_at       TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (transaction_hash, proxy_wallet, token_id, side)
);
CREATE INDEX IF NOT EXISTS data_trades_wallet_ts ON data_trades (proxy_wallet, ts DESC);
CREATE INDEX IF NOT EXISTS data_trades_token_ts  ON data_trades (token_id, ts DESC);

-- Data API /positions. A snapshot of open exposure, so captured_at is in the key.
CREATE TABLE IF NOT EXISTS positions (
    proxy_wallet         TEXT        NOT NULL,
    asset                TEXT        NOT NULL,
    captured_at          TIMESTAMPTZ NOT NULL,
    condition_id         TEXT        NOT NULL,
    size                 NUMERIC     NOT NULL,
    avg_price            NUMERIC     NOT NULL,
    initial_value        NUMERIC     NOT NULL,
    current_value        NUMERIC     NOT NULL,
    cash_pnl             NUMERIC     NOT NULL,
    percent_pnl          NUMERIC     NOT NULL,
    total_bought         NUMERIC     NOT NULL,
    realized_pnl         NUMERIC     NOT NULL,
    percent_realized_pnl NUMERIC     NOT NULL,
    cur_price            NUMERIC     NOT NULL,
    redeemable           BOOLEAN     NOT NULL,
    mergeable            BOOLEAN     NOT NULL,
    outcome              TEXT,
    outcome_index        BIGINT,
    end_date             TEXT,
    negative_risk        BOOLEAN,
    PRIMARY KEY (proxy_wallet, asset, captured_at)
);
CREATE INDEX IF NOT EXISTS positions_wallet_ts ON positions (proxy_wallet, captured_at DESC);

-- Data API /activity.
CREATE TABLE IF NOT EXISTS activity (
    transaction_hash TEXT        NOT NULL,
    proxy_wallet     TEXT        NOT NULL,
    asset            TEXT        NOT NULL,
    activity_type    TEXT        NOT NULL,
    ts               TIMESTAMPTZ NOT NULL,
    condition_id     TEXT        NOT NULL,
    size             NUMERIC     NOT NULL,
    usdc_size        NUMERIC     NOT NULL,
    price            NUMERIC     NOT NULL,
    side             TEXT,
    outcome          TEXT,
    outcome_index    BIGINT,
    title            TEXT,
    slug             TEXT,
    event_slug       TEXT,
    scraped_at       TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (transaction_hash, proxy_wallet, asset, activity_type)
);
CREATE INDEX IF NOT EXISTS activity_wallet_ts ON activity (proxy_wallet, ts DESC);

-- Data API /value. `market` is '' rather than NULL for the portfolio-wide
-- figure, because NULL cannot participate in a primary key.
CREATE TABLE IF NOT EXISTS user_values (
    proxy_wallet TEXT        NOT NULL,
    market       TEXT        NOT NULL DEFAULT '',
    captured_at  TIMESTAMPTZ NOT NULL,
    value        NUMERIC     NOT NULL,
    PRIMARY KEY (proxy_wallet, market, captured_at)
);

-- Data API /holders.
CREATE TABLE IF NOT EXISTS token_holders (
    token         TEXT        NOT NULL,
    proxy_wallet  TEXT        NOT NULL,
    captured_at   TIMESTAMPTZ NOT NULL,
    asset         TEXT        NOT NULL,
    amount        NUMERIC     NOT NULL,
    outcome_index BIGINT,
    PRIMARY KEY (token, proxy_wallet, captured_at)
);
CREATE INDEX IF NOT EXISTS token_holders_token_ts ON token_holders (token, captured_at DESC);

-- /user-pnl.
CREATE TABLE IF NOT EXISTS user_pnl (
    proxy_wallet TEXT        NOT NULL,
    ts           TIMESTAMPTZ NOT NULL,
    pnl          NUMERIC     NOT NULL,
    PRIMARY KEY (proxy_wallet, ts)
);
