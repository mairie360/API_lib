use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};
use std::rc::Rc;

use crate::{database::queries::is_admin_query, pool::AppState};
use crate::{
    database::query_views::IsAdminQueryView,
    jwt_manager::{check_jwt_validity, get_jwt_from_request, get_user_id_from_jwt, JWTCheckError},
};

use crate::security::AuthenticatedUser;

/**
 * Middleware to check the validity of JWT tokens in incoming requests.
 * If the token is valid, the request is passed to the next service in the chain.
 * If the token is invalid or missing, an appropriate HTTP response is returned.
 */
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

    /**
     * Creates a new instance of the middleware service, wrapping the provided service.
     */
    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AdminMiddlewareService {
            service: Rc::new(service),
        }))
    }
}

/**
 * Service that implements the actual logic of checking JWT tokens for each incoming request.
 * It uses the `get_jwt_from_request` function to extract the token and the `check_jwt_validity` function to validate it.
 * Depending on the result, it either forwards the request to the next service or returns an appropriate HTTP response.
 */
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

    /**
     * Handles the incoming request by checking for a JWT token and validating it.
     */
    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();
        let app_state = req.app_data::<actix_web::web::Data<AppState>>();

        // On clone le pool pour la closure async move
        let pool = match app_state {
            Some(state) => state.db_pool.clone(),
            None => None,
        };

        let path = req.path();
        if !path.starts_with("/admin") {
            return Box::pin(async move {
                let res = svc.call(req).await?;
                Ok(res.map_into_left_body())
            });
        }

        Box::pin(async move {
            let pool = match pool {
                Some(p) => p,
                None => {
                    // Erreur si le pool n'a pas été injecté dans l'App
                    let res = HttpResponse::InternalServerError()
                        .body("DB Pool missing")
                        .map_into_right_body();
                    return Ok(req.into_response(res));
                }
            };

            let jwt_option = get_jwt_from_request(req.request());

            let jwt = match jwt_option {
                Some(token) => token,
                None => {
                    let response = HttpResponse::Unauthorized()
                        .body("Unauthorized: No JWT token provided.")
                        .map_into_right_body();
                    return Ok(req.into_response(response));
                }
            };

            match check_jwt_validity(&jwt, pool.clone()).await {
                Ok(_) => {
                    let view: IsAdminQueryView = IsAdminQueryView::new(
                        get_user_id_from_jwt(&jwt).unwrap().parse().unwrap_or(0),
                    );
                    if is_admin_query(view, pool).await.unwrap() {
                        req.extensions_mut().insert(AuthenticatedUser {
                            id: get_user_id_from_jwt(&jwt).unwrap().parse().unwrap_or(0),
                        });

                        let res = svc.call(req).await?;
                        Ok(res.map_into_left_body())
                    } else {
                        let response = HttpResponse::Forbidden()
                            .body("Forbidden: User is not an admin.")
                            .map_into_right_body();
                        Ok(req.into_response(response))
                    }
                }
                Err(error) => {
                    let response =
                        match error {
                            JWTCheckError::DatabaseError => HttpResponse::InternalServerError()
                                .body("Internal server error: Database not initialized."),
                            JWTCheckError::NoTokenProvided => HttpResponse::Unauthorized()
                                .body("Unauthorized: No JWT token provided."),
                            JWTCheckError::ExpiredToken => HttpResponse::Unauthorized()
                                .body("Unauthorized: JWT token is expired."),
                            JWTCheckError::InvalidToken => HttpResponse::Unauthorized()
                                .body("Unauthorized: Invalid JWT token."),
                            JWTCheckError::UnknownUser => {
                                HttpResponse::NotFound().body("User not found.")
                            }
                        };
                    Ok(req.into_response(response.map_into_right_body()))
                }
            }
        })
    }
}
