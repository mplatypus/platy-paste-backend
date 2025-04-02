FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

# Install necessary dependencies for OpenSSL and musl
ENV SQLX_OFFLINE=true

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
# Reinstall dependencies in builder staged

COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release

# We do not need the Rust toolchain to run the binary!
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Copy binary directly instead of full 'bin' directory
COPY --from=builder /app/target/release/platy-paste /usr/local/bin/platy-paste

ENTRYPOINT ["/usr/local/bin/platy-paste"]