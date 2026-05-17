FROM rust:1.95-slim AS chef
WORKDIR /build
RUN apt-get update \
    && apt-get install -y --no-install-recommends musl-tools ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && rustup target add x86_64-unknown-linux-musl \
    && cargo install cargo-chef

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl -p almanac-server

FROM scratch
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/server /server
COPY providers /data/providers
COPY models /data/models
COPY aliases.yaml /data/aliases.yaml
COPY schema /data/schema

ENV DATA_DIR=/data
ENV PORT=8080
ENV RUST_LOG=almanac_server=info,tower_http=info

USER 10001:10001
EXPOSE 8080

CMD ["/server"]
