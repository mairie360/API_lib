use crate::{
    database::{query_views::HasAccessQueryView, SqlId},
    security::AuthenticatedUser,
    state::AppState,
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

/// Middleware vérifiant, via `check_access` en base, que l'utilisateur authentifié a le droit
/// d'effectuer `action` sur la ressource décrite par l'`AccessCheckConfig` de la route.
///
/// # Errors
///
/// - 500 si l'`AccessCheckConfig` ou l'`AppState` est absent de la route, ou si la base échoue ;
/// - 401 si aucun utilisateur authentifié n'a été injecté ;
/// - 400 si l'identifiant de l'URL n'est pas un entier, 404 s'il sort de `1..=i32::MAX` ;
/// - 404 si la ressource n'existe pas, 403 si les droits sont insuffisants.
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
            if val.is_empty() || !val.bytes().all(|b| b.is_ascii_digit()) {
                return Err(actix_web::error::ErrorBadRequest(
                    "Invalid ID format in URL",
                ));
            }
            // An integer Postgres cannot hold names no row (MAIR-422): 404, never an alias.
            let id = val
                .parse::<SqlId>()
                .map_err(|_| actix_web::error::ErrorNotFound("Resource not found"))?;
            instance_id = Some(id.as_u64());
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
