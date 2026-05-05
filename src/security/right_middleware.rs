use actix_web::{
    body::BoxBody,
    dev::{ServiceRequest, ServiceResponse},
    middleware::Next,
};
use actix_web::{Error, HttpMessage};

use crate::{pool::AppState, security::AuthenticatedUser};

#[derive(Clone)]
pub struct AccessCheckConfig {
    /// Le nom de la ressource (ex: "users", "sessions")
    pub resource_name: &'static str,
    /// L'action requise (ex: "read", "write")
    pub action: &'static str,
    /// Le nom du paramètre dans l'URL (ex: "user_id", "id")
    /// Si None, on considère que c'est une vérification globale (instance = NULL)
    pub id_param_pattern: Option<&'static str>,
}

pub async fn access_guard_middleware(
    req: ServiceRequest,
    next: Next<BoxBody>,
) -> Result<ServiceResponse<BoxBody>, Error> {
    // 1. Récupérer la config de la route
    let config = req.app_data::<AccessCheckConfig>().ok_or_else(|| {
        actix_web::error::ErrorInternalServerError("AccessConfig missing on route")
    })?;

    // 2. Récupérer l'utilisateur injecté par JwtMiddleware
    let user = req
        .extensions()
        .get::<AuthenticatedUser>()
        .copied()
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("User not authenticated"))?;

    // 3. Extraire l'ID de l'instance dans l'URL (si défini)
    let mut instance_id: Option<i32> = None;
    if let Some(param_name) = config.id_param_pattern {
        if let Some(val) = req.match_info().get(param_name) {
            instance_id = val.parse::<i32>().ok();
            if instance_id.is_none() {
                return Err(actix_web::error::ErrorBadRequest(
                    "Invalid ID format in URL",
                ));
            }
        }
    }

    // 4. Appel à ta fonction DB
    let app_state = req
        .app_data::<actix_web::web::Data<AppState>>()
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("AppState missing"))?;

    let db_pool = match app_state.db_pool.clone() {
        Some(pool) => pool,
        None => {
            return Err(actix_web::error::ErrorInternalServerError(
                "Database pool missing",
            ))
        }
    };

    // On exécute la requête SQL que tu as écrite
    let is_allowed = sqlx::query_scalar::<_, bool>("SELECT check_access($1, $2, $3, $4)")
        .bind(user.id as i32)
        .bind(config.resource_name)
        .bind(config.action)
        .bind(instance_id)
        .fetch_one(&db_pool)
        .await
        .map_err(|e| {
            eprintln!("{:?}", e);
            actix_web::error::ErrorInternalServerError("Database error during access check")
        })?;

    // 5. Verdict
    if is_allowed {
        next.call(req).await
    } else {
        // Optionnel : Tu pourrais log ici en Rust aussi,
        // mais ta fonction SQL s'en occupe déjà !
        Err(actix_web::error::ErrorForbidden("Insufficient permissions"))
    }
}
