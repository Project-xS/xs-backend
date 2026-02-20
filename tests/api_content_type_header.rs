use actix_web::http::header;
use actix_web::http::StatusCode;
use actix_web::test;
use actix_web::{web, App, HttpResponse};
use proj_xs::api::ContentTypeHeader;

#[actix_rt::test]
async fn content_type_header_accepts_json_variants() {
    let app = test::init_service(
        App::new().service(
            web::resource("/guarded")
                .guard(ContentTypeHeader)
                .route(web::post().to(|| async { HttpResponse::Ok() })),
        ),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/guarded")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req = test::TestRequest::post()
        .uri("/guarded")
        .insert_header((header::CONTENT_TYPE, "application/json; charset=utf-8"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_rt::test]
async fn content_type_header_rejects_missing_or_other_types() {
    let app = test::init_service(
        App::new().service(
            web::resource("/guarded")
                .guard(ContentTypeHeader)
                .route(web::post().to(|| async { HttpResponse::Ok() })),
        ),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/guarded")
        .insert_header((header::CONTENT_TYPE, "text/plain"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let req = test::TestRequest::post().uri("/guarded").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
