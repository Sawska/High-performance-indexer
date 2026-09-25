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
    side      TEXT        NOT NULL,   
    level_idx INT         NOT NULL,   
    price     NUMERIC     NOT NULL,
    size      NUMERIC     NOT NULL,
    PRIMARY KEY (asset_id, ts, hash, side, level_idx)
);

CREATE TABLE IF NOT EXISTS quotes (
    asset_id   TEXT        NOT NULL,
    kind       TEXT        NOT NULL,   
    side       TEXT,                   
    value      NUMERIC     NOT NULL,
    captured_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS quotes_asset_kind_ts
    ON quotes (asset_id, kind, captured_at DESC);

CREATE TABLE IF NOT EXISTS price_history (
    market TEXT        NOT NULL,
    ts     TIMESTAMPTZ NOT NULL,
    price  NUMERIC     NOT NULL,
    PRIMARY KEY (market, ts)
);

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

CREATE TABLE IF NOT EXISTS user_values (
    proxy_wallet TEXT        NOT NULL,
    market       TEXT        NOT NULL DEFAULT '',
    captured_at  TIMESTAMPTZ NOT NULL,
    value        NUMERIC     NOT NULL,
    PRIMARY KEY (proxy_wallet, market, captured_at)
);

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

CREATE TABLE IF NOT EXISTS user_pnl (
    proxy_wallet TEXT        NOT NULL,
    ts           TIMESTAMPTZ NOT NULL,
    pnl          NUMERIC     NOT NULL,
    PRIMARY KEY (proxy_wallet, ts)
);

CREATE INDEX IF NOT EXISTS data_trades_condition_ts ON data_trades (condition_id, ts DESC);
CREATE INDEX IF NOT EXISTS data_trades_ts           ON data_trades (ts DESC);
CREATE INDEX IF NOT EXISTS markets_volume           ON markets (volume_num DESC NULLS LAST);
CREATE INDEX IF NOT EXISTS activity_ts              ON activity (ts DESC);

CREATE TABLE IF NOT EXISTS users (
    id            BIGSERIAL   PRIMARY KEY,
    email         TEXT        NOT NULL UNIQUE,
    password_hash TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS sessions (
    token_hash TEXT        PRIMARY KEY,
    user_id    BIGINT      NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS sessions_user ON sessions (user_id);
