use crate::auth::AdminPrincipal;
use crate::db::{AssetOperations, CanteenOperations, MenuOperations, RepositoryError, S3Error};
use crate::enums::admin::ItemUploadResponse;
use actix_web::http::StatusCode;
use actix_web::{get, post, web, HttpResponse, Responder};

#[utoipa::path(
    tag = "Assets",
    responses(
        (status = 200, description = "Presigned URL generated successfully", body = ItemUploadResponse),
        (status = 500, description = "Unable to generate presigned url", body = ItemUploadResponse)
    ),
    summary = "Generate s3 presigned URL to upload images",
    description = "Generates a s3 presigned URL to upload images with. Do a PUT request with the body as the image binary to the returned URL to upload images."
)]
#[post("/upload/{item_id}")]
pub async fn upload_image_handler(
    menu_ops: web::Data<MenuOperations>,
    admin: AdminPrincipal,
    path: web::Path<(i32,)>,
) -> impl Responder {
    let item_id_val = path.into_inner().0;
    match menu_ops
        .upload_menu_item_pic(&item_id_val, admin.canteen_id)
        .await
    {
        Ok(url) => {
            debug!(
                "upload_image: successfully generated presigned url '{:?}'",
                url
            );
            HttpResponse::Ok().json(ItemUploadResponse {
                status: "ok".to_string(),
                url,
                item_id: item_id_val,
                error: None,
            })
        }
        Err(e) => {
            error!(
                "upload_image: couldn't generate presigned url for item {}: {}",
                item_id_val, e
            );
            let (status, message) = match e {
                RepositoryError::NotFound(_) => {
                    (StatusCode::FORBIDDEN, "item not found".to_string())
                }
                other => (StatusCode::CONFLICT, other.to_string()),
            };
            HttpResponse::build(status).json(ItemUploadResponse {
                status: "error".to_string(),
                url: String::new(),
                item_id: -1,
                error: Some(message),
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
pub async fn get_image_handler(
    asset_ops: web::Data<AssetOperations>,
    menu_ops: web::Data<MenuOperations>,
    canteen_ops: web::Data<CanteenOperations>,
    admin: AdminPrincipal,
    path: web::Path<(String,)>,
) -> impl Responder {
    let s3_key = path.into_inner().0;
    let lookup_key = s3_key
        .strip_prefix("items/")
        .or_else(|| s3_key.strip_prefix("canteens/"))
        .unwrap_or(&s3_key)
        .to_string();

    let owner_canteen_id = admin.canteen_id;
    let lookup_key_for_menu = lookup_key.clone();
    let owned_by_menu_item = match web::block(move || {
        menu_ops.canteen_owns_menu_item_pic_key(owner_canteen_id, &lookup_key_for_menu)
    })
    .await
    {
        Ok(Ok(owned)) => owned,
        Ok(Err(e)) => {
            error!(
                "get_image: error checking menu pic ownership for key '{}' and canteen {}: {}",
                lookup_key, owner_canteen_id, e
            );
            return HttpResponse::InternalServerError().json(ItemUploadResponse {
                status: "error".to_string(),
                url: String::new(),
                item_id: -1,
                error: Some(e.to_string()),
            });
        }
        Err(e) => {
            error!(
                "get_image: blocking error checking menu pic ownership for key '{}' and canteen {}: {}",
                lookup_key, owner_canteen_id, e
            );
            return HttpResponse::InternalServerError().json(ItemUploadResponse {
                status: "error".to_string(),
                url: String::new(),
                item_id: -1,
                error: Some(e.to_string()),
            });
        }
    };

    let owned_by_canteen_pic = if owned_by_menu_item {
        false
    } else {
        let lookup_key_for_canteen = lookup_key.clone();
        match web::block(move || {
            canteen_ops.canteen_owns_canteen_pic_key(owner_canteen_id, &lookup_key_for_canteen)
        })
        .await
        {
            Ok(Ok(owned)) => owned,
            Ok(Err(e)) => {
                error!(
                    "get_image: error checking canteen pic ownership for key '{}' and canteen {}: {}",
                    lookup_key, owner_canteen_id, e
                );
                return HttpResponse::InternalServerError().json(ItemUploadResponse {
                    status: "error".to_string(),
                    url: String::new(),
                    item_id: -1,
                    error: Some(e.to_string()),
                });
            }
            Err(e) => {
                error!(
                    "get_image: blocking error checking canteen pic ownership for key '{}' and canteen {}: {}",
                    lookup_key, owner_canteen_id, e
                );
                return HttpResponse::InternalServerError().json(ItemUploadResponse {
                    status: "error".to_string(),
                    url: String::new(),
                    item_id: -1,
                    error: Some(e.to_string()),
                });
            }
        }
    };

    if !(owned_by_menu_item || owned_by_canteen_pic) {
        return HttpResponse::Forbidden().json(ItemUploadResponse {
            status: "error".to_string(),
            url: String::new(),
            item_id: -1,
            error: Some("key not found".to_string()),
        });
    }

    match asset_ops.get_object_presign(&s3_key).await {
        Ok(url) => {
            debug!(
                "get_image: successfully generated presigned url '{:?}'",
                url
            );
            HttpResponse::Ok().json(ItemUploadResponse {
                status: "ok".to_string(),
                url,
                item_id: -1,
                error: None,
            })
        }
        Err(e) => {
            error!("get_image: couldn't generate presigned url '{:?}'", s3_key);
            match e {
                S3Error::NotFound(_) => HttpResponse::NotFound().json(ItemUploadResponse {
                    status: "error".to_string(),
                    url: String::new(),
                    item_id: -1,
                    error: Some(String::from("key not found")),
                }),
                _ => HttpResponse::InternalServerError().json(ItemUploadResponse {
                    status: "error".to_string(),
                    url: String::new(),
                    item_id: -1,
                    error: Some(e.to_string()),
                }),
            }
        }
    }
}
