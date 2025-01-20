use actix_web::error::JsonPayloadError;
use actix_web::{Error, HttpRequest, HttpResponse};

pub(crate) fn default_error_handler(err: JsonPayloadError, req: &HttpRequest) -> Error {
    error!("Error in request: {} \n Error: {}", req.full_url(), err);
    actix_web::error::InternalError::from_response("", HttpResponse::BadRequest().finish()).into()
}
