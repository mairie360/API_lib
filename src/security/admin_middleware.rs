use crate::security::AuthenticatedUser;
use crate::{
    database::query_views::IsAdminQueryView,
    jwt_manager::{check_jwt_validity, get_jwt_from_request, get_user_id_from_jwt},
    state::AppState,
};
use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use lazy_static::lazy_static;
use regex::Regex;
use std::future::{ready, Ready};
use std::rc::Rc;

pub struct AdminMiddleware;

impl<S, B> Transform<S, ServiceRequest> for AdminMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = AdminMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AdminMiddlewareService {
            service: Rc::new(service),
        }))
    }
}

pub struct AdminMiddlewareService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AdminMiddlewareService<S>
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

        let path = req.path().to_string();
        Box::pin(async move {
            let db_interface = app_state.get_smart_db();
            lazy_static! {
                static ref ADMIN_PATH_REGEX: Regex = Regex::new(r"/api/v\d+/admin").unwrap();
            }

            if !ADMIN_PATH_REGEX.is_match(&path) {
                let res = svc.call(req).await?;
                return Ok(res.map_into_left_body());
            }

            let jwt = get_jwt_from_request(req.request()).ok_or_else(|| {
                actix_web::error::ErrorUnauthorized("Unauthorized: No JWT token provided.")
            })?;

            check_jwt_validity(&jwt, &db_interface)
                .await
                .map_err(actix_web::Error::from)?;

            let user_id_str = get_user_id_from_jwt(&jwt).ok_or_else(|| {
                actix_web::error::ErrorUnauthorized("Unauthorized: Invalid token payload.")
            })?;

            let user_id = user_id_str.parse().unwrap_or(0);
            let view = IsAdminQueryView::new(user_id);

            let is_admin = db_interface
                .fetch_scalar::<bool, _>(&view)
                .await
                .map_err(actix_web::Error::from)?;

            if is_admin {
                req.extensions_mut()
                    .insert(AuthenticatedUser { id: user_id });
                let res = svc.call(req).await?;
                Ok(res.map_into_left_body())
            } else {
                Err(actix_web::error::ErrorForbidden(
                    "Forbidden: User is not an admin.",
                ))
            }
        })
    }
}
