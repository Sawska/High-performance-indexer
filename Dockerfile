FROM node:24-bookworm-slim AS web

WORKDIR /usr/src/web

COPY web/package.json web/package-lock.json ./
RUN npm ci

COPY web ./
RUN npm run build

FROM rust:1-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
    && echo 'fn main() {}' > src/main.rs \
    && cargo build --release \
    && rm -rf src target/release/pred-market-indexer*

COPY src ./src
COPY schema.sql schema_clickhouse.sql ./

RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --system --create-home --uid 10001 indexer

COPY --from=builder \
    /usr/src/app/target/release/pred-market-indexer \
    /usr/local/bin/pred-market-indexer

COPY --from=web /usr/src/web/dist /usr/local/share/pred-market-indexer/web
ENV WEB_DIR=/usr/local/share/pred-market-indexer/web

USER indexer
WORKDIR /home/indexer

ENTRYPOINT ["pred-market-indexer"]
