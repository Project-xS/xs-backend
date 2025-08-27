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

FROM deps-builder as cert-builder

RUN apt-get update -qqq && \
    apt-get install -yqqq ca-certificates && \
    rm -rf /var/lib/apt/lists/* && \
    update-ca-certificates

# Stage 3: Runtime image
FROM gcr.io/distroless/cc-debian12:nonroot as runtime
COPY --from=app-builder /app/target/release/proj-xs ./proj-xs
COPY --from=cert-builder /etc/ssl/certs /etc/ssl/certs
EXPOSE 8080
CMD ["./proj-xs"]
