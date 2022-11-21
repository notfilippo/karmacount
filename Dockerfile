FROM rust:1 AS chef
WORKDIR /karmacount

RUN cargo install cargo-chef

FROM chef AS planner

COPY . .

RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /karmacount/recipe.json recipe.json

RUN cargo chef cook --release --recipe-path recipe.json

COPY . .

RUN cargo build --release --bin karmacount

FROM debian:buster-slim AS runtime
COPY --from=builder /karmacount/target/release/karmacount /usr/local/bin
WORKDIR /

RUN apt-get update && apt-get install -y ca-certificates libfontconfig1-dev
RUN update-ca-certificates

USER root

ENTRYPOINT ["/usr/local/bin/karmacount"]
