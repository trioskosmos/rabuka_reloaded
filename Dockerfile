# ============================================================
# Stage 1: Build the Rust engine
# ============================================================
FROM rust:slim-bookworm AS rust-builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build/engine

COPY engine/Cargo.toml engine/Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release --bin rabuka_engine 2>/dev/null || true && \
    rm -rf src

COPY engine/ ./
RUN cargo build --release --bin rabuka_engine

# ============================================================
# Stage 2: Minimal runtime image
# ============================================================
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=rust-builder /build/engine/target/release/rabuka_engine  /app/engine/rabuka_engine
COPY web_ui/                                                          /app/web_ui/
COPY cards/cards.json                                                 /app/cards/cards.json
COPY cards/abilities.json                                             /app/cards/abilities.json
COPY web_ui/decks/                                                    /app/game/decks/

ENV PORT=7860
ENV RUST_LOG=warn

EXPOSE 7860

WORKDIR /app/engine
CMD ["./rabuka_engine", "web-server"]
