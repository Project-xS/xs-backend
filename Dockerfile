FROM rust:bookworm as builder

RUN mkdir /app
WORKDIR /app

ADD Cargo.toml /app
ADD Cargo.lock /app
ADD src /app/src

RUN cargo build --release

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=builder /app/target/release/proj-xs ./proj-xs
EXPOSE 8080
CMD ["./proj-xs"]
