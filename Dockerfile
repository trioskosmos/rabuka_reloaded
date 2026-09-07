# ============================================================
# Stage 1: Build the Rust engine
# ============================================================
FROM rust:slim-bookworm AS rust-builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    python3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy entire repo
COPY . .

WORKDIR /build/engine

# Copy Cargo files first for dependency caching
COPY engine/Cargo.toml engine/Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release --features server --bin rabuka_engine 2>/dev/null || true && \
    rm -rf src

# Copy engine source
COPY engine/ ./

# Generate cards_gen.rs and abilities build artifacts (from engine dir, using ../cards)
RUN python3 ../cards/compile_cards.py && python3 ../cards/compile_abilities.py

# Generate per-deck card blobs (required by include_bytes! in decks_cards_gen.rs)
RUN python3 ../tools/bake_deck_cards.py

# Build the actual binary
RUN cargo build --release --features server --bin rabuka_engine

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
# Baked deck blobs (compiled by tools/bake_deck_cards.py)
COPY --from=rust-builder /build/engine/baked/decks/                   /app/engine/baked/decks/

ENV PORT=8080
ENV RUST_LOG=info

EXPOSE 8080

WORKDIR /app/engine
CMD ["./rabuka_engine", "web-server"]
