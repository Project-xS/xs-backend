use log::{debug, error};
use crate::db::SearchOperations;
use crate::enums::admin::AllItemsResponse;
use actix_web::{get, web, HttpResponse, Responder};

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
) -> impl Responder {
    let search_query = &path.into_inner().0;
    match search_ops.search_menu_items(&search_query.clone()) {
        Ok(x) => {
            debug!("get_search_query_results: successfully executed fuzzy search for query '{}'", search_query);
            HttpResponse::Ok().json(AllItemsResponse {
                status: "ok".to_string(),
                data: x,
                error: None,
            })
        }
        Err(e) => {
            error!("get_search_query_results: fuzzy search failed for query '{}': {}", search_query, e);
            HttpResponse::BadRequest().json(AllItemsResponse {
                status: "error".to_string(),
                data: Vec::new(),
                error: Some(e.to_string()),
            })
        }
    }
}
