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

-- ────────────────────────────────────────────────────────────────────────────
-- On-chain (Polygon) indexer
-- ────────────────────────────────────────────────────────────────────────────

-- One row per feed; `block_hash` is the hash of `last_block`, used to detect
-- reorgs that reach back past the confirmation depth.
CREATE TABLE IF NOT EXISTS chain_cursors (
    name       TEXT PRIMARY KEY,
    last_block BIGINT      NOT NULL,
    block_hash TEXT,
    updated_at TIMESTAMPTZ NOT NULL
);

-- Block timestamps, cached so a block is never fetched from the RPC twice.
CREATE TABLE IF NOT EXISTS chain_blocks (
    number BIGINT PRIMARY KEY,
    hash   TEXT        NOT NULL,
    ts     TIMESTAMPTZ NOT NULL
);

-- CTF Exchange OrderFilled / OrdersMatched, both v1 and v2 layouts.
CREATE TABLE IF NOT EXISTS chain_order_fills (
    transaction_hash    TEXT        NOT NULL,
    log_index           BIGINT      NOT NULL,
    block_number        BIGINT      NOT NULL,
    block_time          TIMESTAMPTZ,
    exchange            TEXT        NOT NULL,
    exchange_version    INT         NOT NULL,
    neg_risk            BOOLEAN     NOT NULL,
    event               TEXT        NOT NULL,  -- order_filled | orders_matched
    order_hash          TEXT        NOT NULL,
    maker               TEXT,
    taker               TEXT,
    side                TEXT,                  -- BUY | SELL, from the maker's side
    token_id            TEXT,                  -- exact uint256, joins markets.clob_token_ids
    maker_asset_id      TEXT,
    taker_asset_id      TEXT,
    maker_amount_filled NUMERIC,
    taker_amount_filled NUMERIC,
    fee                 NUMERIC,
    price               NUMERIC,
    size                NUMERIC,
    builder             TEXT,                  -- v2 only
    metadata            TEXT,                  -- v2 only
    PRIMARY KEY (transaction_hash, log_index)
);
CREATE INDEX IF NOT EXISTS chain_order_fills_token_ts ON chain_order_fills (token_id, block_time DESC);
CREATE INDEX IF NOT EXISTS chain_order_fills_maker_ts ON chain_order_fills (maker, block_time DESC);
CREATE INDEX IF NOT EXISTS chain_order_fills_block    ON chain_order_fills (block_number DESC);
CREATE INDEX IF NOT EXISTS chain_order_fills_ts       ON chain_order_fills (block_time DESC);

-- ConditionalTokens PositionSplit / PositionsMerge.
CREATE TABLE IF NOT EXISTS chain_ctf_positions (
    transaction_hash     TEXT        NOT NULL,
    log_index            BIGINT      NOT NULL,
    block_number         BIGINT      NOT NULL,
    block_time           TIMESTAMPTZ,
    kind                 TEXT        NOT NULL,  -- split | merge
    stakeholder          TEXT        NOT NULL,
    collateral_token     TEXT,
    parent_collection_id TEXT,
    condition_id         TEXT        NOT NULL,
    partition_ids        JSONB       NOT NULL,
    amount               NUMERIC,
    PRIMARY KEY (transaction_hash, log_index)
);
CREATE INDEX IF NOT EXISTS chain_ctf_positions_cond   ON chain_ctf_positions (condition_id, block_time DESC);
CREATE INDEX IF NOT EXISTS chain_ctf_positions_holder ON chain_ctf_positions (stakeholder, block_time DESC);
CREATE INDEX IF NOT EXISTS chain_ctf_positions_block  ON chain_ctf_positions (block_number DESC);

-- ConditionalTokens PayoutRedemption.
CREATE TABLE IF NOT EXISTS chain_ctf_redemptions (
    transaction_hash     TEXT        NOT NULL,
    log_index            BIGINT      NOT NULL,
    block_number         BIGINT      NOT NULL,
    block_time           TIMESTAMPTZ,
    redeemer             TEXT        NOT NULL,
    collateral_token     TEXT,
    parent_collection_id TEXT,
    condition_id         TEXT        NOT NULL,
    index_sets           JSONB       NOT NULL,
    payout               NUMERIC,
    PRIMARY KEY (transaction_hash, log_index)
);
CREATE INDEX IF NOT EXISTS chain_ctf_redemptions_cond     ON chain_ctf_redemptions (condition_id, block_time DESC);
CREATE INDEX IF NOT EXISTS chain_ctf_redemptions_redeemer ON chain_ctf_redemptions (redeemer, block_time DESC);
CREATE INDEX IF NOT EXISTS chain_ctf_redemptions_block    ON chain_ctf_redemptions (block_number DESC);

-- ConditionPreparation and ConditionResolution collapsed into one row per
-- condition; preparation creates it, resolution fills the payout columns.
CREATE TABLE IF NOT EXISTS chain_conditions (
    condition_id       TEXT PRIMARY KEY,
    oracle             TEXT,
    question_id        TEXT,
    outcome_slot_count BIGINT,
    prepared_block     BIGINT,
    prepared_at        TIMESTAMPTZ,
    prepared_tx        TEXT,
    resolved_block     BIGINT,
    resolved_at        TIMESTAMPTZ,
    resolved_tx        TEXT,
    payout_numerators  JSONB
);
CREATE INDEX IF NOT EXISTS chain_conditions_resolved ON chain_conditions (resolved_at DESC NULLS LAST);
CREATE INDEX IF NOT EXISTS chain_conditions_blocks   ON chain_conditions (prepared_block DESC);

-- ERC-1155 TransferSingle / TransferBatch on the ConditionalTokens contract.
-- A batch transfer fans out to one row per (id, value) pair via item_index.
CREATE TABLE IF NOT EXISTS chain_token_transfers (
    transaction_hash TEXT        NOT NULL,
    log_index        BIGINT      NOT NULL,
    item_index       INT         NOT NULL,
    block_number     BIGINT      NOT NULL,
    block_time       TIMESTAMPTZ,
    contract         TEXT        NOT NULL,
    operator         TEXT,
    from_address     TEXT        NOT NULL,
    to_address       TEXT        NOT NULL,
    token_id         TEXT        NOT NULL,
    amount           NUMERIC,
    PRIMARY KEY (transaction_hash, log_index, item_index)
);
CREATE INDEX IF NOT EXISTS chain_token_transfers_token ON chain_token_transfers (token_id, block_time DESC);
CREATE INDEX IF NOT EXISTS chain_token_transfers_from  ON chain_token_transfers (from_address, block_time DESC);
CREATE INDEX IF NOT EXISTS chain_token_transfers_to    ON chain_token_transfers (to_address, block_time DESC);
CREATE INDEX IF NOT EXISTS chain_token_transfers_block ON chain_token_transfers (block_number DESC);

-- ERC-20 Transfer on the collateral tokens, filtered to logs where one side is
-- a watched Polymarket contract.
CREATE TABLE IF NOT EXISTS chain_collateral_transfers (
    transaction_hash TEXT        NOT NULL,
    log_index        BIGINT      NOT NULL,
    block_number     BIGINT      NOT NULL,
    block_time       TIMESTAMPTZ,
    token            TEXT        NOT NULL,
    symbol           TEXT,
    from_address     TEXT        NOT NULL,
    to_address       TEXT        NOT NULL,
    amount           NUMERIC,
    PRIMARY KEY (transaction_hash, log_index)
);
CREATE INDEX IF NOT EXISTS chain_collateral_transfers_from  ON chain_collateral_transfers (from_address, block_time DESC);
CREATE INDEX IF NOT EXISTS chain_collateral_transfers_to    ON chain_collateral_transfers (to_address, block_time DESC);
CREATE INDEX IF NOT EXISTS chain_collateral_transfers_block ON chain_collateral_transfers (block_number DESC);

-- Token id -> market, so on-chain rows can name their market with an indexed
-- lookup. `markets.clob_token_ids` holds a JSON array *as text*, and matching a
-- token inside it with LIKE costs a full scan of markets per row.
CREATE TABLE IF NOT EXISTS market_tokens (
    token_id     TEXT PRIMARY KEY,
    market_id    TEXT NOT NULL,
    condition_id TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS market_tokens_condition ON market_tokens (condition_id);

-- Backfill for markets stored before this table existed. The scraper keeps it
-- up to date from then on. Split with string functions rather than a jsonb cast
-- so a malformed value can never abort schema setup.
INSERT INTO market_tokens (token_id, market_id, condition_id)
SELECT btrim(tok, ' "'), m.id, m.condition_id
  FROM markets m,
       LATERAL unnest(string_to_array(btrim(m.clob_token_ids, '[]'), ',')) AS tok
 WHERE m.clob_token_ids LIKE '[%]'
   AND btrim(tok, ' "') <> ''
ON CONFLICT (token_id) DO NOTHING;
