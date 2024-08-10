FROM rust:1
COPY Cargo.lock Cargo.toml ./
COPY src/ src/
RUN cargo build --release
CMD ["./target/release/rvterminal"]