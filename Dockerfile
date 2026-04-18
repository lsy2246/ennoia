# syntax=docker/dockerfile:1.6

# -------- build --------
FROM rust:1.95-slim AS build
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Layer cache: copy manifests first
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY migrations ./migrations

RUN cargo build --release --bin ennoia

# -------- runtime --------
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Non-root user for runtime safety
RUN useradd --system --create-home --home-dir /home/ennoia --shell /usr/sbin/nologin ennoia

COPY --from=build /app/target/release/ennoia /usr/local/bin/ennoia
COPY --from=build /app/crates/cli/templates /opt/ennoia/templates

ENV ENNOIA_HOME=/data/ennoia
WORKDIR /data

# Runtime state volume (SQLite lives here)
RUN mkdir -p /data/ennoia && chown -R ennoia:ennoia /data /home/ennoia /opt/ennoia

USER ennoia

EXPOSE 3710

# On first start, `init` populates the home from the baked-in templates;
# subsequent starts reuse the existing state. `serve` then runs the server.
CMD ["sh", "-c", "ennoia init /data/ennoia && ennoia serve /data/ennoia"]
