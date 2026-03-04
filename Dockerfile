FROM rust:1.84-bookworm AS builder

WORKDIR /usr/src/mojo

# Cache dependencies by building a dummy project first
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

# Build the real binary
COPY src ./src
RUN touch src/main.rs && cargo build --release

# Runtime image with C/C++ toolchain pre-installed
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    clang \
    clang-format \
    git \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/mojo/target/release/mojo /usr/local/bin/mojo

WORKDIR /workspace
ENTRYPOINT ["mojo"]
