FROM rust:1.91-alpine AS builder
RUN apk add --no-cache musl-dev protobuf-dev
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

COPY src ./src
# Hacemos un touch a main.rs para forzar reconstrucción
RUN touch src/main.rs && cargo build --release

FROM alpine:latest
WORKDIR /app
COPY --from=builder /app/target/release/rust-service ./
EXPOSE 8081
CMD ["./rust-service"]