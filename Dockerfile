# Use the official Rust image as the base image
FROM rust:1.88 as builder

# Set the working directory
WORKDIR /app

# Copy the Cargo.toml and Cargo.lock files
COPY Cargo.toml Cargo.lock ./
COPY .sqlx ./.sqlx

# Create a dummy main.rs to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this will be cached unless Cargo.toml changes)
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin chalkbyte
RUN rm src/main.rs

# Copy the source code
COPY src ./src
COPY migrations ./migrations

# Build the application
RUN touch src/main.rs && cargo build --release --bin chalkbyte

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -m -u 1001 chalkbyte

# Set the working directory
WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/chalkbyte .

# Copy migrations
COPY --from=builder /app/migrations ./migrations

# Change ownership to the chalkbyte user
RUN chown -R chalkbyte:chalkbyte /app
USER chalkbyte

# Expose the port
EXPOSE 3000

# Set environment variables
ENV RUST_LOG=info
ENV PORT=3000

# Run the application
CMD ["./chalkbyte"]
