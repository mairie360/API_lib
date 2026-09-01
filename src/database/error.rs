// Fichier : src/database/error.rs

use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Erreur interne : {0}")]
    Internal(String),

    #[error("Erreur de correspondance du DTO : {0}")]
    MappingError(String),

    #[error("Violation de contrainte d'unicité (doublon) : {0}")]
    UniqueViolation(String),

    #[error("Violation de clé étrangère : {0}")]
    ForeignKeyViolation(String),

    #[error("Ressource non trouvée")]
    NotFound,

    #[error("Erreur de base de données : {0}")]
    Sqlx(sqlx::Error),
}

impl From<sqlx::Error> for DbError {
    fn from(err: sqlx::Error) -> Self {
        match &err {
            sqlx::Error::RowNotFound => DbError::NotFound,
            sqlx::Error::Database(db_err) => {
                if let Some(code) = db_err.code() {
                    match code.as_ref() {
                        "23505" => return DbError::UniqueViolation(db_err.message().to_string()),
                        "23503" => {
                            return DbError::ForeignKeyViolation(db_err.message().to_string())
                        }
                        _ => {}
                    }
                }
                DbError::Sqlx(err)
            }
            _ => DbError::Sqlx(err),
        }
    }
}

impl ResponseError for DbError {
    fn status_code(&self) -> StatusCode {
        match self {
            DbError::NotFound => StatusCode::NOT_FOUND,
            DbError::UniqueViolation(_) => StatusCode::CONFLICT,
            DbError::ForeignKeyViolation(_) => StatusCode::BAD_REQUEST,
            DbError::MappingError(_) | DbError::Internal(_) | DbError::Sqlx(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    fn error_response(&self) -> HttpResponse {
        // --- LOGS AUTOMATIQUES SELON LA CRITICITÉ ---
        match self {
            // Cas bénins / erreurs utilisateurs : simple trace informative (ou rien du tout)
            DbError::NotFound => {
                // Pas besoin de logger en erreur, c'est un comportement utilisateur classique
            }
            DbError::UniqueViolation(msg) => {
                // Optionnel : un avertissement pour savoir qu'un doublon a été tenté
                eprintln!("[AVERTISSEMENT DB] Tentative de doublon : {}", msg);
            }
            DbError::ForeignKeyViolation(msg) => {
                eprintln!("[AVERTISSEMENT DB] Référence invalide : {}", msg);
            }

            // Vrais problèmes techniques (Erreurs 500) : Log critique indispensable
            DbError::MappingError(msg) => {
                eprintln!(
                    "[ERREUR CRITIQUE DB] Échec du mapping JSON vers DTO : {}",
                    msg
                );
            }
            DbError::Internal(msg) => {
                eprintln!("[ERREUR CRITIQUE DB] Erreur interne : {}", msg);
            }
            DbError::Sqlx(err) => {
                eprintln!("[ERREUR CRITIQUE DB] Erreur de pilote SQLx : {:?}", err);
            }
        }

        // --- GÉNÉRATION DE LA RÉPONSE HTTP ---
        match self {
            DbError::NotFound => HttpResponse::NotFound().body("Ressource non trouvée"),
            DbError::UniqueViolation(msg) => {
                HttpResponse::Conflict().body(format!("Conflit de données : {}", msg))
            }
            DbError::ForeignKeyViolation(msg) => {
                HttpResponse::BadRequest().body(format!("Référence invalide : {}", msg))
            }
            DbError::MappingError(msg) => {
                HttpResponse::InternalServerError().body(format!("Erreur de mapping : {}", msg))
            }
            _ => HttpResponse::InternalServerError().body("Erreur interne de la base de données"),
        }
    }
}
