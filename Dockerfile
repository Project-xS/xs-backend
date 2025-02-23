# Cache deps
FROM public.ecr.aws/docker/library/rust:bookworm as deps-builder

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
FROM gcr.io/distroless/cc-debian12:nonroot as runtime
COPY --from=app-builder /app/target/release/proj-xs ./proj-xs
EXPOSE 8080
CMD ["./proj-xs"]
