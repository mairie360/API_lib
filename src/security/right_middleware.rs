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

    // 4. Appel à la base de données
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

    // Mise à jour ici : On récupère un i32 au lieu d'un bool
    let access_status = sqlx::query_scalar::<_, i32>("SELECT check_access($1, $2, $3, $4)")
        .bind(user.id as i32)
        .bind(config.resource_name)
        .bind(config.action)
        .bind(instance_id)
        .fetch_one(&db_pool)
        .await
        .map_err(|e| {
            eprintln!("Access check SQL error: {:?}", e);
            actix_web::error::ErrorInternalServerError("Database error during access check")
        })?;

    // 5. Verdict étendu selon le code retourné par la DB
    match access_status {
        1 => {
            // Accès accordé, on passe au handler ou middleware suivant
            next.call(req).await
        }
        -1 => {
            // La ressource ou la table n'existe pas -> 404 Not Found propre
            Err(actix_web::error::ErrorNotFound("Resource not found"))
        }
        _ => {
            // Pas de droits (0) ou toute autre valeur -> 403 Forbidden standard
            Err(actix_web::error::ErrorForbidden("Insufficient permissions"))
        }
    }
}
