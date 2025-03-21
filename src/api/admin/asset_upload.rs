use actix_web::{get, post, web, HttpResponse, Responder};
use uuid::Uuid;
use crate::db::AssetUploadOperations;
use crate::enums::admin::ItemUploadResponse;

#[utoipa::path(
    tag = "Assets",
    responses(
        (status = 200, description = "Presigned URL generated successfully", body = ItemUploadResponse),
        (status = 500, description = "Unable to generate presigned url", body = ItemUploadResponse)
    ),
    summary = "Generate s3 presigned URL to upload images",
    description = "Generates a s3 presigned URL to upload images with. Do a PUT request with the body as the image binary to the returned URL to upload images."
)]
#[post("/upload")]
pub async fn upload_image_handler(asset_ops: web::Data<AssetUploadOperations>) -> impl Responder {
    let s3_key = Uuid::new_v4().to_string();
    match asset_ops.upload_object(&s3_key).await {
        Ok(url) => {
            debug!(
                "upload_image: successfully generated presigned url '{:?}'",
                url
            );
            HttpResponse::Ok().json(ItemUploadResponse {
                status: "ok".to_string(),
                url: url.to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!(
                "upload_image: couldn't generate presigned url '{:?}'", s3_key,
            );
            HttpResponse::InternalServerError().json(ItemUploadResponse {
                status: "error".to_string(),
                url: String::new(),
                error: Some(e.to_string()),
            })
        }
    }
}

#[utoipa::path(
    tag = "Assets",
    params(
        ("key", description = "The unique identifier image to fetch"),
    ),
    responses(
        (status = 200, description = "Canteen successfully created", body = ItemUploadResponse),
        (status = 400, description = "Failed to create canteen: invalid request or data error", body = ItemUploadResponse)
    ),
    summary = "Generate s3 presigned URL to retrieve images",
    description = "Generates a s3 presigned URL to retrieve images with. Do a GET request to the returned URL to fetch images.",

)]
#[get("/{key}")]
pub async fn get_image_handler(asset_ops: web::Data<AssetUploadOperations>, path: web::Path<(String, )>) -> impl Responder {
    let s3_key = path.into_inner().0;
    match asset_ops.get_object(&s3_key).await {
        Ok(url) => {
            debug!(
                "get_image: successfully generated presigned url '{:?}'",
                url
            );
            HttpResponse::Ok().json(ItemUploadResponse {
                status: "ok".to_string(),
                url: url.to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!(
                "get_image: couldn't generate presigned url '{:?}'", s3_key,
            );
            HttpResponse::InternalServerError().json(ItemUploadResponse {
                status: "error".to_string(),
                url: String::new(),
                error: Some(e.to_string()),
            })
        }
    }
}
