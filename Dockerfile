# Build stage
FROM rust:1.83-slim as builder

WORKDIR /usr/src/app

# Install required system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libudev-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY config ./config
COPY style ./style
COPY templates ./templates

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libudev1 \
    libssl3 \
    makemkv-bin \
    makemkv-oss \
    handbrake-cli \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /usr/src/app/target/release/torn /app/torn
COPY --from=builder /usr/src/app/config /app/config
COPY --from=builder /usr/src/app/style /app/style
COPY --from=builder /usr/src/app/templates /app/templates

# Create directories for data
RUN mkdir -p /app/data/raw /app/data/output

# Expose the web interface port
EXPOSE 8080

# Run the application
CMD ["./torn", "rip"]