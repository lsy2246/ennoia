# syntax=docker/dockerfile:1.6

# -------- api build --------
FROM rust:1.95-slim AS api-build
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY assets ./assets
COPY crates ./crates

RUN cargo build --release --bin ennoia

# -------- api runtime --------
FROM debian:bookworm-slim AS api

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --system --create-home --home-dir /home/ennoia --shell /usr/sbin/nologin ennoia

COPY --from=api-build /app/target/release/ennoia /usr/local/bin/ennoia

ENV ENNOIA_HOME=/data/ennoia
WORKDIR /data

RUN mkdir -p /data/ennoia && chown -R ennoia:ennoia /data /home/ennoia

USER ennoia

EXPOSE 3710

CMD ["sh", "-c", "ennoia init \"$ENNOIA_HOME\" && sed -i 's/^host = \".*\"/host = \"0.0.0.0\"/' \"$ENNOIA_HOME/config/server.toml\" && ennoia serve \"$ENNOIA_HOME\""]

# -------- web build --------
FROM oven/bun:1 AS web-build
WORKDIR /app

COPY .npmrc package.json bun.lock ./
COPY web ./web

RUN bun install --frozen-lockfile
RUN bun run --cwd web/apps/shell build

# -------- web runtime --------
FROM nginx:alpine AS web

COPY --from=web-build /app/web/apps/shell/dist /usr/share/nginx/html

EXPOSE 80
