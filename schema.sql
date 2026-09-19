-- Polymarket firehose tables.
-- Apply with:  psql "$DATABASE_URL" -f schema.sql

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
