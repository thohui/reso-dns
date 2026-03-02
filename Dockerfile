# Build stage, needs Rust + Node.js for embedding the web UI assets into the binary
FROM rust:1.88-bookworm AS builder

# build.rs runs pnpm install + build, so we need Node.js and pnpm
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y nodejs \
    && corepack enable && corepack prepare pnpm@latest --activate

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY reso/ reso/

RUN cargo build --release

# Runtime stage 
FROM debian:bookworm-slim AS runtime

# install ca-certificates for HTTPS support
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# create a non-root user to run the server
RUN useradd -r -s /usr/sbin/nologin reso

COPY --from=builder /build/target/release/reso /usr/local/bin/reso

# bind to all interfaces so docker port mapping works
ENV RESO_DNS_SERVER_ADDRESS=0.0.0.0:53
ENV RESO_HTTP_SERVER_ADDRESS=0.0.0.0:80

EXPOSE 53/tcp 53/udp 80/tcp

USER reso

ENTRYPOINT ["/usr/local/bin/reso"]
