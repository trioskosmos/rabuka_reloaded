# ============================================================
# Stage 1: Build the frontend
# ============================================================
FROM node:20-slim AS frontend-builder

WORKDIR /build/web_ui

COPY web_ui/package.json web_ui/package-lock.json ./
RUN npm ci --prefer-offline

COPY web_ui/ ./
RUN npm run build

# ============================================================
# Stage 2: Build the Rust engine
# ============================================================
FROM rust:1.78-slim AS rust-builder

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build/engine

# Cache dependency build layer separately
COPY engine/Cargo.toml engine/Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release --bin rabuka_engine 2>/dev/null || true && \
    rm -rf src

# Now build the real source
COPY engine/ ./
RUN cargo build --release --bin rabuka_engine

# ============================================================
# Stage 3: Minimal runtime image
# ============================================================
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Directory layout mirrors what the binary expects:
# Binary runs from /app/engine/ so relative paths resolve:
#   ../cards/cards.json  →  /app/cards/cards.json
#   ../game/decks/       →  /app/game/decks/
#   ../web_ui/dist/      →  /app/web_ui/dist/

WORKDIR /app

COPY --from=rust-builder   /build/engine/target/release/rabuka_engine  /app/engine/rabuka_engine
COPY --from=frontend-builder /build/web_ui/dist                        /app/web_ui/dist/
COPY cards/cards.json                                                   /app/cards/cards.json
COPY game/decks/                                                        /app/game/decks/

# Hugging Face Spaces uses port 7860
ENV PORT=7860
ENV RUST_LOG=warn

EXPOSE 7860

# Run the binary from /app/engine so relative paths resolve correctly
WORKDIR /app/engine
CMD ["./rabuka_engine", "web-server"]
