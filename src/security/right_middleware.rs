use crate::{
    database::query_views::HasAccessQueryView, security::AuthenticatedUser, state::AppState,
};
use actix_web::{
    body::BoxBody,
    dev::{ServiceRequest, ServiceResponse},
    middleware::Next,
    Error, HttpMessage,
};

#[derive(Clone)]
pub struct AccessCheckConfig {
    pub resource_name: &'static str,
    pub action: &'static str,
    pub id_param_pattern: Option<&'static str>,
}

pub async fn access_guard_middleware(
    req: ServiceRequest,
    next: Next<BoxBody>,
) -> Result<ServiceResponse<BoxBody>, Error> {
    let config = req.app_data::<AccessCheckConfig>().ok_or_else(|| {
        actix_web::error::ErrorInternalServerError("AccessConfig missing on route")
    })?;

    let user = req
        .extensions()
        .get::<AuthenticatedUser>()
        .copied()
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("User not authenticated"))?;

    let mut instance_id: Option<u64> = None;
    if let Some(param_name) = config.id_param_pattern {
        if let Some(val) = req.match_info().get(param_name) {
            instance_id = val.parse::<u64>().ok();
            if instance_id.is_none() {
                return Err(actix_web::error::ErrorBadRequest(
                    "Invalid ID format in URL",
                ));
            }
        }
    }

    let app_state = req
        .app_data::<actix_web::web::Data<AppState>>()
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("AppState missing"))?;
    let db_interface = app_state.get_smart_db();

    let view = HasAccessQueryView::new(user.id, config.resource_name, config.action, instance_id);

    // L'erreur de la SmartDatabase (ApiLibError) est convertie proprement en actix_web::Error
    let access_status = db_interface
        .fetch_scalar::<i32, _>(&view)
        .await
        .map_err(actix_web::Error::from)?;

    match access_status {
        1 => next.call(req).await,
        -1 => Err(actix_web::error::ErrorNotFound("Resource not found")),
        _ => Err(actix_web::error::ErrorForbidden("Insufficient permissions")),
    }
}
