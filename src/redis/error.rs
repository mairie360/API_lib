// Fichier : src/redis/error.rs[cite: 4]

use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RedisError {
    #[error("Erreur de pool Redis : {0}")]
    Pool(String),
    #[error("Erreur du driver Redis : {0}")]
    Driver(String),
    #[error("Erreur interne Redis : {0}")]
    Internal(String),
    #[error("Erreur de valeur Redis : {0}")]
    Value(String),
}

impl ResponseError for RedisError {
    // 1. Définition explicite du code HTTP 500 pour les problèmes d'infrastructure cache
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    // 2. Génération de la réponse avec logs automatiques selon la nature de l'erreur
    fn error_response(&self) -> HttpResponse {
        match self {
            RedisError::Pool(msg) => {
                eprintln!(
                    "[ERREUR CRITIQUE REDIS] Échec du pool de connexions : {}",
                    msg
                );
            }
            RedisError::Driver(msg) => {
                eprintln!("[ERREUR CRITIQUE REDIS] Erreur du driver lors de l'exécution d'une commande : {}", msg);
            }
            RedisError::Internal(msg) => {
                eprintln!("[ERREUR CRITIQUE REDIS] Erreur interne : {}", msg);
            }
            RedisError::Value(msg) => {
                eprintln!(
                    "[AVERTISSEMENT REDIS] Problème de désérialisation ou de valeur : {}",
                    msg
                );
            }
        }

        HttpResponse::InternalServerError().body(format!("Erreur cache : {}", self))
    }
}
