use crate::jwt_manager::{check_jwt_validity, get_jwt_from_request, get_user_id_from_jwt};
use crate::security::AuthenticatedUser;
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

        let path = req.path();
        if path == "/"
            || path.starts_with("/swagger-ui")
            || path.starts_with("/api-docs")
            || path.contains("/auth")
        {
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

            // L'erreur JWT est convertie automatiquement en actix_web::Error grâce à ResponseError
            check_jwt_validity(&jwt, &db_interface)
                .await
                .map_err(actix_web::Error::from)?;

            let user_id = get_user_id_from_jwt(&jwt)
                .and_then(|id| id.parse().ok())
                .unwrap_or(0);

            req.extensions_mut()
                .insert(AuthenticatedUser { id: user_id });

            let res = svc.call(req).await?;
            Ok(res.map_into_left_body())
        })
    }
}
