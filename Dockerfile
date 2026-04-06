# Stage 1: Build the Rust backend
FROM rust:1.87 AS rust-builder
WORKDIR /build

# Copy manifests to cache dependencies
COPY Cargo.toml Cargo.lock ./
COPY server/Cargo.toml server/Cargo.toml

# Build dependencies only (dummy main)
RUN mkdir -p server/src && echo "fn main() {}" > server/src/main.rs \
    && cargo build --release -p ootle-community-templates

# Copy source and build final binary
COPY server/src/ server/src/
COPY migrations/ migrations/
COPY .sqlx/ .sqlx/
ENV SQLX_OFFLINE=true
RUN cargo build --release -p ootle-community-templates

# Stage 2: Build the frontend
FROM node:22-alpine AS web-builder
WORKDIR /build
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ .
RUN npm run build

# Stage 3: Final image
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=rust-builder /build/target/release/ootle-community-templates ./
COPY --from=web-builder /build/dist ./static/
COPY migrations/ migrations/
EXPOSE 3000
CMD ["./ootle-community-templates", "--config", "config.toml"]
