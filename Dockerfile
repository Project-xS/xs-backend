# Cache deps
FROM rust:bookworm as deps-builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./

# Dummy source to cache deps
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN cargo build --release

# Build app
FROM deps-builder as app-builder

COPY src ./src
COPY migrations ./migrations
RUN touch src/main.rs
RUN cargo build --release

# Stage 3: Runtime image
FROM debian:bookworm as runtime
COPY --from=app-builder /app/target/release/proj-xs ./proj-xs
EXPOSE 8080
CMD ["./proj-xs"]
