# --- Stage 1: Build Recipe ---
FROM rust:1.96-slim AS builder

WORKDIR /app

# Install build essentials required for compiling some dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy configuration manifests
COPY Cargo.toml Cargo.lock ./

# Create a dummy source file to build dependencies and leverage Docker caching layers
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy real code base and migrations folder
COPY src ./src
COPY migrations ./migrations

# Update the dummy build artifacts timestamp to force re-compilation of our actual code
RUN touch src/main.rs
RUN cargo build --release

# --- Stage 2: Runtime Final Package ---
FROM debian:bookworm-slim AS runtime

WORKDIR /app

# Install OpenSSL runtime dependency required by sqlx / native-tls
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy compiled binary from build engine stage
COPY --from=builder /app/target/release/assesment-rust ./assessment-rust
COPY --from=builder /app/migrations ./migrations

# Expose port and configure environment runtime profiles
EXPOSE 8080
ENV RUST_LOG=info

CMD ["./assessment-rust"]
