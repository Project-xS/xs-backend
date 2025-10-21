use crate::auth::principal::Principal;
use actix_web::dev::Payload;
use actix_web::{error::ErrorUnauthorized, Error, FromRequest, HttpMessage, HttpRequest};
use futures::future::{ready, Ready};

pub struct PrincipalExtractor(pub Principal);

impl FromRequest for PrincipalExtractor {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        if let Some(p) = req.extensions().get::<Principal>() {
            return ready(Ok(PrincipalExtractor(p.clone())));
        }
        ready(Err(ErrorUnauthorized("missing principal")))
    }
}

pub struct UserPrincipal {
    user_id: i32,
    firebase_uid: String,
    email: Option<String>,
}

impl UserPrincipal {
    pub fn user_id(&self) -> i32 {
        self.user_id
    }
    #[allow(dead_code)]
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }
}

impl FromRequest for UserPrincipal {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        if let Some(p) = req.extensions().get::<Principal>() {
            if let Principal::User {
                user_id,
                firebase_uid,
                email,
            } = p.clone()
            {
                return ready(Ok(UserPrincipal {
                    user_id,
                    firebase_uid,
                    email,
                }));
            }
            return ready(Err(actix_web::error::ErrorForbidden("admin not allowed")));
        }
        ready(Err(ErrorUnauthorized("missing principal")))
    }
}

pub struct AdminPrincipal {
    pub canteen_id: i32,
}

impl FromRequest for AdminPrincipal {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        if let Some(p) = req.extensions().get::<Principal>() {
            if let Principal::Admin { canteen_id } = p.clone() {
                return ready(Ok(AdminPrincipal { canteen_id }));
            }
            return ready(Err(actix_web::error::ErrorForbidden("user not allowed")));
        }
        ready(Err(ErrorUnauthorized("missing principal")))
    }
}
