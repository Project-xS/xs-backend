use crate::auth::UserPrincipal;
use crate::sse::SseBroker;
use actix_web::{get, web, Responder};
use actix_web_lab::sse;
use actix_web_lab::sse::Sse;
use std::time::Duration;
use uuid::Uuid;

#[utoipa::path(
    tag = "User",
    responses(
        (status = 200, description = "Successfully connect to SSE stream", content_type = "text/event-stream"),
        (status = 401, description = "Auth token missing"),
        (status = 500, description = "Failed to connect to SSE stream"),
    ),
    summary = "Connect to SSE stream for user order updates",
)]
#[get("/orders")]
pub async fn user_order_events(
    user: UserPrincipal,
    broker: web::Data<SseBroker>,
) -> impl Responder {
    let user_id = user.user_id();
    let conn_id = Uuid::new_v4();

    let (tx, rx) = tokio::sync::mpsc::channel::<sse::Event>(64);
    broker.register_user_connection(user_id, conn_id, tx.clone());

    let _ = tx
        .send(sse::Data::new("connected").event("status").into())
        .await;

    let cleanup_broker = broker.clone();
    actix_web::rt::spawn(async move {
        let _ = tx.closed().await;
        cleanup_broker.unregister_user_connection(user_id, conn_id);
    });

    Sse::from_infallible_receiver(rx)
        .with_retry_duration(Duration::from_secs(3))
        .with_keep_alive(Duration::from_secs(10))
}
