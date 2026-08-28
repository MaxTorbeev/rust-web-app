FROM rust:1.94.1-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    cargo build \
      --locked \
      --release \
      --package mxt-realtime \
    && cp /app/target/release/mxt-realtime /tmp/mxt-realtime


FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder \
    --chown=10001:10001 \
    /tmp/mxt-realtime \
    /usr/local/bin/mxt-realtime

ENV APP_URL=0.0.0.0:4008
ENV RUST_LOG=info

USER 10001:10001

EXPOSE 4008

ENTRYPOINT ["/usr/local/bin/mxt-realtime"]
