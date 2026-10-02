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
            sqlx::Error::RowNotFound => Self::NotFound,
            sqlx::Error::Database(db_err) => {
                if let Some(code) = db_err.code() {
                    match code.as_ref() {
                        "23505" => return Self::UniqueViolation(db_err.message().to_string()),
                        "23503" => return Self::ForeignKeyViolation(db_err.message().to_string()),
                        _ => {}
                    }
                }
                Self::Sqlx(err)
            }
            _ => Self::Sqlx(err),
        }
    }
}

impl ResponseError for DbError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::UniqueViolation(_) => StatusCode::CONFLICT,
            Self::ForeignKeyViolation(_) => StatusCode::BAD_REQUEST,
            Self::MappingError(_) | Self::Internal(_) | Self::Sqlx(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    fn error_response(&self) -> HttpResponse {
        // --- LOGS AUTOMATIQUES SELON LA CRITICITÉ ---
        match self {
            // Cas bénins / erreurs utilisateurs : simple trace informative (ou rien du tout)
            Self::NotFound => {
                // Pas besoin de logger en erreur, c'est un comportement utilisateur classique
            }
            Self::UniqueViolation(msg) => {
                // Optionnel : un avertissement pour savoir qu'un doublon a été tenté
                eprintln!("[AVERTISSEMENT DB] Tentative de doublon : {msg}");
            }
            Self::ForeignKeyViolation(msg) => {
                eprintln!("[AVERTISSEMENT DB] Référence invalide : {msg}");
            }

            // Vrais problèmes techniques (Erreurs 500) : Log critique indispensable
            Self::MappingError(msg) => {
                eprintln!("[ERREUR CRITIQUE DB] Échec du mapping JSON vers DTO : {msg}");
            }
            Self::Internal(msg) => {
                eprintln!("[ERREUR CRITIQUE DB] Erreur interne : {msg}");
            }
            Self::Sqlx(err) => {
                eprintln!("[ERREUR CRITIQUE DB] Erreur de pilote SQLx : {err:?}");
            }
        }

        // --- HTTP response: generic bodies only. Postgres messages name tables, columns and
        // constraints, and are logged above instead of being sent to the client (MAIR-391). ---
        let body = match self {
            Self::NotFound => "Resource not found",
            Self::UniqueViolation(_) => "Data conflict: the resource already exists",
            Self::ForeignKeyViolation(_) => "Invalid reference to another resource",
            Self::MappingError(_) | Self::Internal(_) | Self::Sqlx(_) => "Internal database error",
        };
        HttpResponse::build(self.status_code()).body(body)
    }
}
