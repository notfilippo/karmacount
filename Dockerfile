FROM messense/rust-musl-cross:aarch64-musl AS chef

WORKDIR /karmacount
USER root

RUN cargo install cargo-chef

FROM chef AS planner

COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

COPY --from=planner /karmacount/recipe.json recipe.json

RUN apt-get update && apt-get install -y ca-certificates && update-ca-certificates

RUN cargo chef cook --release --target aarch64-unknown-linux-musl --recipe-path recipe.json
COPY . ./
RUN cargo build --release --target aarch64-unknown-linux-musl --bin karmacount

FROM alpine AS runtime

COPY --from=builder /karmacount/target/aarch64-unknown-linux-musl/release/karmacount /karmacount
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

USER root

CMD [ "./karmacount" ]
