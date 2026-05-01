# ── Build stage ───────────────────────────────────────────────────────────────
FROM rust:1.78-slim AS builder

WORKDIR /build
COPY . .
RUN cargo build --release -p almanac-server

# ── Runtime stage ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/server /usr/local/bin/server

# Data directory — mount the almanac repo here at runtime
ENV DATA_DIR=/data
ENV PORT=8080

EXPOSE 8080

CMD ["server"]
