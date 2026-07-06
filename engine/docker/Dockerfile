FROM nandolawson/rust-3ds:latest

WORKDIR /work

# Copy workspace into container
COPY . /work

WORKDIR /work/engine

# Build harness without server features
RUN cargo build --bin harness --release --no-default-features || true

CMD ["/bin/bash"]
