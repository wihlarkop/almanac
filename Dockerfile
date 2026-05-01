# ── Chef base (cargo-chef + musl target pre-installed) ────────────────────────
FROM rust:1.95.0-slim AS chef
WORKDIR /build
RUN apt-get update \
    && apt-get install -y --no-install-recommends musl-tools \
    && rm -rf /var/lib/apt/lists/* \
    && rustup target add x86_64-unknown-linux-musl \
    && cargo install cargo-chef

# ── Planner: extract dependency recipe from Cargo files ───────────────────────
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ── Builder: cache deps first, then compile app ───────────────────────────────
FROM chef AS builder
COPY --from=planner /build/recipe.json recipe.json
# This layer is cached as long as dependencies don't change
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl -p almanac-server

# ── Runtime: zero-OS scratch image ────────────────────────────────────────────
FROM scratch
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/server /server

ENV DATA_DIR=/data
ENV PORT=8080

EXPOSE 8080

CMD ["/server"]
