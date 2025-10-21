use std::future::{ready, Ready};
use std::rc::Rc;

use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{error::ErrorUnauthorized, http::header, Error, HttpMessage};
use futures::future::LocalBoxFuture;

use crate::auth::admin_jwt::verify_admin_jwt;
use crate::auth::config::{AdminJwtConfig, FirebaseAuthConfig};
use crate::auth::firebase::verify_firebase_token;
use crate::auth::jwks::JwksCache;
use crate::auth::Principal;
use crate::db::UserOperations;

#[derive(Clone)]
pub struct AuthLayer {
    firebase_cfg: FirebaseAuthConfig,
    admin_cfg: AdminJwtConfig,
    jwks: JwksCache,
    user_ops: UserOperations,
}

impl AuthLayer {
    pub fn new(
        firebase_cfg: FirebaseAuthConfig,
        admin_cfg: AdminJwtConfig,
        jwks: JwksCache,
        user_ops: UserOperations,
    ) -> Self {
        Self {
            firebase_cfg,
            admin_cfg,
            jwks,
            user_ops,
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for AuthLayer
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddleware {
            service: Rc::new(service),
            inner: self.clone(),
        }))
    }
}

pub struct AuthMiddleware<S> {
    service: Rc<S>,
    inner: AuthLayer,
}

impl<S, B> Service<ServiceRequest> for AuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Bypass only '/', '/health', and '/canteen/login'
        let path = req.path().to_string();
        if path == "/" || path == "/health" || path == "/canteen/login" {
            let fut = self.service.call(req);
            #[allow(clippy::redundant_async_block)]
            return Box::pin(async move { fut.await });
        }

        let token_opt = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .map(|s| s.to_string());
        if token_opt.as_deref().unwrap_or("").is_empty() {
            return Box::pin(async { Err(ErrorUnauthorized("missing or invalid auth header")) });
        }

        let token = token_opt.unwrap();
        let inner = self.inner.clone();
        let srv = self.service.clone();
        Box::pin(async move {
            // 1) Try admin JWT
            if let Ok(canteen_id) = verify_admin_jwt(&token, &inner.admin_cfg) {
                req.extensions_mut().insert(Principal::Admin { canteen_id });
                return srv.call(req).await;
            }

            // 2) Try Firebase token
            if let Ok(v) = verify_firebase_token(&token, &inner.firebase_cfg, &inner.jwks).await {
                let uid = v.uid.clone();
                let email = v.email.clone();
                let display = v.display_name.clone();
                let photo = v.photo_url.clone();
                let verified = v.email_verified;

                let user_ops = inner.user_ops.clone();
                let uid_for_principal = uid.clone();
                let email_for_principal = email.clone();
                let upsert_res = actix_web::web::block(move || {
                    user_ops.upsert_firebase_user(uid, email, display, photo, verified)
                })
                .await;

                return match upsert_res {
                    Ok(Ok(user)) => {
                        req.extensions_mut().insert(Principal::User {
                            user_id: user.user_id,
                            firebase_uid: uid_for_principal,
                            email: email_for_principal,
                        });
                        srv.call(req).await
                    }
                    _ => Err(ErrorUnauthorized("user upsert failed")),
                };
            }

            Err(ErrorUnauthorized("unauthorized"))
        })
    }
}
