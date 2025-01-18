FROM rust:bookworm as builder

RUN mkdir /app
WORKDIR /app

COPY Cargo.toml /app/Cargo.toml
COPY Cargo.lock /app/Cargo.lock
COPY src /app/src

RUN cargo build --release --locked

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=builder /app/target/release/proj-xs ./proj-xs
EXPOSE 8080
CMD ["./proj-xs"]
