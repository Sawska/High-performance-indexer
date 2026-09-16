FROM rust:latest

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./

COPY src ./src

RUN cargo build --release

CMD [ "./pred-market-indexer" ]