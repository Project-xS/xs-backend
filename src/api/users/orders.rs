use crate::auth::UserPrincipal;
use crate::db::UserOperations;
use crate::enums::users::PastOrdersItemResponse;
use actix_web::{get, web, HttpResponse, Responder};

#[utoipa::path(
    tag = "User",
    responses(
        (status = 200, description = "Successfully retrieved order items for the specified user or RFID", body = PastOrdersItemResponse),
        (status = 500, description = "Failed to retrieve order items due to server error", body = PastOrdersItemResponse),
    ),
    summary = "Get past orders of specified user id",
)]
#[get("/get_past_orders")]
pub(super) async fn get_past_orders_of_user(
    user_ops: web::Data<UserOperations>,
    user: UserPrincipal,
) -> actix_web::Result<impl Responder> {
    let search_user_id = user.user_id();
    let result = user_ops.get_past_orders_by_userid(&search_user_id).await;
    match result {
        Ok(data) => {
            debug!(
                "get_orders_by_user: retrieved {} orders for user_id {}",
                data.len(),
                search_user_id
            );
            Ok(HttpResponse::Ok().json(PastOrdersItemResponse {
                status: "ok".to_string(),
                data,
                error: None,
            }))
        }
        Err(e) => {
            error!(
                "get_orders_by_user: error retrieving orders for user_id {}: {}",
                search_user_id, e
            );
            Ok(
                HttpResponse::InternalServerError().json(PastOrdersItemResponse {
                    status: "error".to_string(),
                    data: Vec::new(),
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}
