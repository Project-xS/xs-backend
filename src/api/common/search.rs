use crate::db::SearchOperations;
use crate::enums::admin::AllItemsResponse;
use actix_web::{get, web, HttpResponse, Responder};

#[utoipa::path(
    get,
    tag = "Search",
    path = "/{query}",
    params(
        ("query", description = "Item name to search for"),
    ),
    responses(
        (status = 200, description = "Searched items result for given search query", body = AllItemsResponse)
    ),
    summary = "Search for items on menu"
)]
#[get("/{query}")]
pub(super) async fn get_search_query_results(search_ops: web::Data<SearchOperations>, path: web::Path<(String,)>) -> impl Responder {
    let search_query = &path.into_inner().0;
    match search_ops.search_menu_items(&search_query.clone()) {
        Ok(x) => {
            debug!("Search query executed: {}", search_query.clone());
            HttpResponse::Ok().json(AllItemsResponse {
                status: "ok".to_string(),
                data: x,
                error: None,
            })
        }
        Err(e) => {
            error!("MENU: get_search_query() failed: {}", search_query.clone());
            HttpResponse::BadRequest().json(AllItemsResponse {
                status: "error".to_string(),
                data: Vec::new(),
                error: Some(e.to_string()),
            })
        }
    }
}
