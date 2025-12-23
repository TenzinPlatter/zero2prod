FROM lukemathwalker/cargo-chef:latest-rust-1.92.0 as chef

WORKDIR /app
RUN apt-get update && apt-get install -y lld clang
RUN cargo install bunyan

FROM chef AS planner

COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

COPY --from=planner /app/recipe.json recipe.json

RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
ENV SQLX_OFFLINE=true

RUN cargo build --release --bin zero2prod

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/zero2prod /usr/local/bin/zero2prod
COPY /configuration /configuration
COPY --from=builder /app/scripts/prod.sh /usr/local/bin/prod.sh
COPY --from=builder /usr/local/cargo/bin/bunyan /usr/local/bin/bunyan

RUN apt-get update -y \
  && apt-get install -y --no-install-recommends openssl ca-certificates \
  # Clean up
  && apt-get autoremove -y \
  && apt-get clean -y \
  && rm -rf /var/lib/apt/lists/*

ENTRYPOINT ["/usr/local/bin/prod.sh"]
