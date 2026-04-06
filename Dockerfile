# Stage 1: Build the Rust backend
FROM rust:1.87 AS rust-builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY server/ server/
COPY migrations/ migrations/
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
