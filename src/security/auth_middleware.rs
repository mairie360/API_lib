use crate::jwt_manager::{authenticate_token, get_jwt_from_request};
use crate::security::{is_public_path, AuthenticatedUser};
use crate::state::AppState;
use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};
use std::rc::Rc;

pub struct JwtMiddleware;

impl<S, B> Transform<S, ServiceRequest> for JwtMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = JwtMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtMiddlewareService {
            service: Rc::new(service),
        }))
    }
}

pub struct JwtMiddlewareService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for JwtMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();
        let app_state = req
            .app_data::<actix_web::web::Data<AppState>>()
            .cloned()
            .unwrap();

        if is_public_path(req.path()) {
            return Box::pin(async move {
                let res = svc.call(req).await?;
                Ok(res.map_into_left_body())
            });
        }

        Box::pin(async move {
            let db_interface = app_state.get_smart_db();

            let jwt = get_jwt_from_request(req.request()).ok_or_else(|| {
                actix_web::error::ErrorUnauthorized("Unauthorized: No JWT token provided.")
            })?;

            // Historical `JWT_SECRET` token or Keycloak access token, picked from the `alg`
            // header; the JWT error is turned into an actix_web::Error through ResponseError.
            let user_id = authenticate_token(&jwt, db_interface, app_state.get_keycloak())
                .await
                .map_err(actix_web::Error::from)?;

            req.extensions_mut()
                .insert(AuthenticatedUser { id: user_id });

            let res = svc.call(req).await?;
            Ok(res.map_into_left_body())
        })
    }
}
