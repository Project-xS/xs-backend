# Backend for Proj-xS

## Setup

1. Get rusty 🦀!
2. Create a PostgreSQL database (migrations will be taken care of automatically!)
3. Copy `.env.example` to `.env` and configure your database URL (Refer below).
4. Run with `cargo run`.

## Environment Variables

- `DATABASE_URL`: PostgreSQL connection string
- `RUST_LOG`: Logging level (default: "info")

- **Asset handling ENV:**
  - `S3_ACCESS_KEY_ID`: Any AWS S3 compatible bucket access key
  - `S3_SECRET_KEY`: Any AWS S3 compatible bucket secret key
  - `S3_ENDPOINT`: AWS S3 compatible endpoint
  - `S3_BUCKET_NAME`: AWS S3 bucket name
  - `S3_REGION`: AWS S3 bucket region

## License

AGPL v3
