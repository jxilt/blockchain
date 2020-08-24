# A very unoptimised Dockerfile.

# Select image.
FROM rust:1.45.2

# Copy your source tree.
COPY ./ ./

# Build for release.
RUN cargo build --release

# Set the startup command to run your binary. Uses the default address (0.0.0.0:10005).
CMD ["./target/release/blockchain"]