use crate::security::AuthenticatedUser;
use crate::{
    database::query_views::IsAdminQueryView,
    jwt_manager::{authenticate_token, get_jwt_from_request},
    state::AppState,
};
use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use regex::Regex;
use std::future::{ready, Ready};
use std::rc::Rc;
use std::sync::LazyLock;

static ADMIN_PATH_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"/api/v\d+/admin").expect("regex des routes admin invalide"));

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

            if !ADMIN_PATH_REGEX.is_match(&path) {
                let res = svc.call(req).await?;
                return Ok(res.map_into_left_body());
            }

            let jwt = get_jwt_from_request(req.request()).ok_or_else(|| {
                actix_web::error::ErrorUnauthorized("Unauthorized: No JWT token provided.")
            })?;

            // Historical `JWT_SECRET` token or Keycloak access token, picked from the `alg` header.
            let user_id = authenticate_token(&jwt, db_interface, app_state.get_keycloak())
                .await
                .map_err(actix_web::Error::from)?;
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
