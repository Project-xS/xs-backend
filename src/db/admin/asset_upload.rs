use crate::db::errors::S3Error;
use aws_config::meta::region::RegionProviderChain;
use aws_config::Region;
use aws_sdk_s3::config::{Builder as S3ConfigBuilder, Credentials};
use aws_sdk_s3::presigning::PresigningConfig;
use std::time::Duration;

pub struct AssetUploadOperations {
    pub client: aws_sdk_s3::Client,
    pub s3_endpoint: String,
    pub bucket_name: String,
}

impl AssetUploadOperations {
    pub async fn new() -> Result<Self, S3Error> {
        let s3_endpoint = std::env::var("S3_ENDPOINT").expect("S3_ENDPOINT must be set");
        let region_provider = RegionProviderChain::first_try(Region::new(
            std::env::var("S3_REGION").expect("S3_REGION must be set"),
        ));
        let access_key_id =
            std::env::var("S3_ACCESS_KEY_ID").expect("S3_ACCESS_KEY_ID must be set");
        let secret_key = std::env::var("S3_SECRET_KEY").expect("S3_SECRET_KEY must be set");
        let bucket_name = std::env::var("S3_BUCKET_NAME").expect("S3_BUCKET_NAME must be set");
        let creds = Credentials::new(&access_key_id, &secret_key, None, None, "custom-provider");

        let config = aws_config::from_env()
            .credentials_provider(creds)
            .endpoint_url(&s3_endpoint)
            .region(region_provider)
            .load()
            .await;

        let s3_config = S3ConfigBuilder::from(&config)
            .force_path_style(true)
            .build();
        let client = aws_sdk_s3::Client::from_conf(s3_config);
        Ok(Self {
            client,
            s3_endpoint,
            bucket_name,
        })
    }

    pub async fn upload_object(&self, key: &str) -> Result<String, S3Error> {
        let response = self
            .client
            .put_object()
            .bucket(&self.bucket_name)
            .key(key)
            .presigned(
                PresigningConfig::builder()
                    .expires_in(Duration::from_secs(6 * 50))
                    .build()
                    .expect("can't build presigning config"),
            )
            .await
            .map_err(S3Error::from)?;

        response.uri();

        Ok(response.uri().to_string())
    }

    pub async fn get_object(&self, key: &str) -> Result<String, S3Error> {
        let response = self
            .client
            .get_object()
            .bucket(&self.bucket_name)
            .key(key)
            .presigned(
                PresigningConfig::builder()
                    .expires_in(Duration::from_secs(6 * 50))
                    .build()
                    .expect("can't build presigning config"),
            )
            .await
            .map_err(S3Error::from)?;

        response.uri();

        Ok(response.uri().to_string())
    }
}

impl Clone for AssetUploadOperations {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            s3_endpoint: self.s3_endpoint.clone(),
            bucket_name: self.bucket_name.clone(),
        }
    }
}
