# https://github.com/LukeMathWalker/cargo-chef

FROM rust:1 AS chef
RUN apt update && apt install -y clang && apt clean
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin iota-sponsoring-service

FROM debian:trixie-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/iota-sponsoring-service /usr/local/bin/

ENTRYPOINT ["/usr/local/bin/iota-sponsoring-service"]
