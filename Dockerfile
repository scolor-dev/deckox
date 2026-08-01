# syntax=docker/dockerfile:1

FROM node:24-bookworm-slim AS web-builder
WORKDIR /usr/src/deckox/apps/web
COPY apps/web/package*.json ./
RUN npm ci
COPY apps/web ./
RUN npm run build

FROM rust:1.85-bookworm AS rust-builder
WORKDIR /usr/src/deckox
COPY Cargo.toml Cargo.lock ./
COPY apps/agent/Cargo.toml apps/agent/Cargo.toml
COPY apps/server/Cargo.toml apps/server/Cargo.toml
COPY crates/protocol/Cargo.toml crates/protocol/Cargo.toml
COPY apps/agent/src apps/agent/src
COPY apps/server/src apps/server/src
COPY crates/protocol/src crates/protocol/src
RUN cargo build --locked --release --package deckox-server --package deckox-agent

FROM debian:bookworm-slim
RUN groupadd --system --gid 10001 deckox \
    && useradd --system --uid 10001 --gid deckox --no-create-home deckox
COPY --from=rust-builder /usr/src/deckox/target/release/deckox-server /usr/local/bin/
COPY --from=rust-builder /usr/src/deckox/target/release/deckox-agent /usr/local/bin/
COPY --from=web-builder /usr/src/deckox/apps/web/dist /usr/local/share/deckox/web
USER deckox:deckox
EXPOSE 8080
ENV DECKOX_LISTEN_ADDR=0.0.0.0:8080 \
    DECKOX_WEB_DIR=/usr/local/share/deckox/web \
    DECKOX_AGENT_SOCKET=/tmp/deckox-agent.sock \
    DECKOX_TERMINAL_ENABLED=true \
    DECKOX_TERMINAL_HOME=/tmp
ENTRYPOINT ["/usr/local/bin/deckox-server"]
