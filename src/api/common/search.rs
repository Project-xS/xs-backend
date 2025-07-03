use crate::db::SearchOperations;
use crate::enums::admin::AllItemsResponse;
use actix_web::{get, web, HttpResponse, Responder};
use log::{debug, error};

#[utoipa::path(
    tag = "Search",
    params(
        ("query", description = "The search query used to perform a fuzzy match on menu item names."),
    ),
    responses(
        (status = 200, description = "Successfully retrieved fuzzy search results for menu items", body = AllItemsResponse)
    ),
    summary = "Perform fuzzy search on menu items"
)]
#[get("/{query}")]
pub(super) async fn get_search_query_results(
    search_ops: web::Data<SearchOperations>,
    path: web::Path<(String,)>,
) -> actix_web::Result<impl Responder> {
    let search_query = path.into_inner().0;
    let search_query_cl = search_query.clone();
    let result = web::block(move || search_ops.search_menu_items(&search_query_cl)).await?;
    match result {
        Ok(x) => {
            debug!(
                "get_search_query_results: successfully executed fuzzy search for query '{}'",
                search_query
            );
            Ok(HttpResponse::Ok().json(AllItemsResponse {
                status: "ok".to_string(),
                data: x,
                error: None,
            }))
        }
        Err(e) => {
            error!(
                "get_search_query_results: fuzzy search failed for query '{}': {}",
                search_query, e
            );
            Ok(HttpResponse::BadRequest().json(AllItemsResponse {
                status: "error".to_string(),
                data: Vec::new(),
                error: Some(e.to_string()),
            }))
        }
    }
}

#[utoipa::path(
    tag = "Search",
    params(
        ("query", description = "The search query used to perform a fuzzy match on menu item names."),
    ),
    responses(
        (status = 200, description = "Successfully retrieved fuzzy search results for menu items", body = AllItemsResponse)
    ),
    summary = "Perform fuzzy search on menu items"
)]
#[get("/{canteen_id}/{query}")]
pub(super) async fn search_query_by_canteen(
    search_ops: web::Data<SearchOperations>,
    path: web::Path<(i32, String)>,
) -> actix_web::Result<impl Responder> {
    let (canteen_id, search_query) = path.into_inner();
    let search_query_cl = search_query.clone();
    let result =
        web::block(move || search_ops.search_menu_items_by_canteen(&canteen_id, &search_query_cl))
            .await?;
    match result {
        Ok(x) => {
            debug!(
                "search_query_by_canteen: successfully executed fuzzy search for canteen {} query '{}'",
                canteen_id, search_query
            );
            Ok(HttpResponse::Ok().json(AllItemsResponse {
                status: "ok".to_string(),
                data: x,
                error: None,
            }))
        }
        Err(e) => {
            error!(
                "search_query_by_canteen: fuzzy search failed for canteen {} query '{}': {}",
                canteen_id, search_query, e
            );
            Ok(HttpResponse::BadRequest().json(AllItemsResponse {
                status: "error".to_string(),
                data: Vec::new(),
                error: Some(e.to_string()),
            }))
        }
    }
}
