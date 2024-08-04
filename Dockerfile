FROM rust:1-slim
COPY Cargo.lock Cargo.toml ./
COPY src/ .
RUN cargo build --release
