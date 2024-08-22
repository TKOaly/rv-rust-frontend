FROM rust:alpine AS builder
WORKDIR /app
RUN apk add build-base libusb-dev
COPY Cargo.lock Cargo.toml ./
COPY ascii/ ascii/
COPY src/ src/
RUN cargo build --release

FROM alpine
WORKDIR /app
COPY --from=builder /app/target/release/rvterminal .
CMD ["./rvterminal"]