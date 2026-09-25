CREATE TABLE IF NOT EXISTS trades
(
    asset_id         LowCardinality(String),
    market           LowCardinality(String),
    ts               DateTime64(3, 'UTC') CODEC(Delta, ZSTD(1)),
    price            Float64              CODEC(Gorilla, ZSTD(1)),
    size             Nullable(Float64)    CODEC(Gorilla, ZSTD(1)),
    side             LowCardinality(String),
    fee_rate_bps     Nullable(Float64),
    transaction_hash String               CODEC(ZSTD(1))
)
ENGINE = ReplacingMergeTree
PARTITION BY toYYYYMM(ts)
ORDER BY (asset_id, ts, transaction_hash);

CREATE TABLE IF NOT EXISTS price_changes
(
    asset_id LowCardinality(String),
    market   LowCardinality(String),
    ts       DateTime64(3, 'UTC') CODEC(Delta, ZSTD(1)),
    price    Float64              CODEC(Gorilla, ZSTD(1)),
    size     Float64              CODEC(Gorilla, ZSTD(1)),
    side     LowCardinality(String),
    best_ask Nullable(Float64)    CODEC(Gorilla, ZSTD(1)),
    hash     String               CODEC(ZSTD(1))
)
ENGINE = ReplacingMergeTree
PARTITION BY toYYYYMM(ts)
ORDER BY (asset_id, ts, hash);

CREATE TABLE IF NOT EXISTS tick_size_changes
(
    asset_id      LowCardinality(String),
    market        LowCardinality(String),
    ts            DateTime64(3, 'UTC') CODEC(Delta, ZSTD(1)),
    old_tick_size Nullable(Float64),
    new_tick_size Float64
)
ENGINE = ReplacingMergeTree
PARTITION BY toYYYYMM(ts)
ORDER BY (asset_id, ts);
