use crate::database::queries::does_user_exist_by_id_query;
use crate::database::query_views::DoesUserExistByIdQueryView;
use crate::jwt_manager::get_timeout_from_jwt;
use crate::jwt_manager::get_user_id_from_jwt;
use crate::jwt_manager::verify_jwt_timeout;
use sqlx::PgPool;

#[derive(Debug, PartialEq)]
pub enum JWTCheckError {
    DatabaseError,
    NoTokenProvided,
    ExpiredToken,
    InvalidToken,
    UnknownUser,
}

pub async fn check_jwt_validity(jwt: &str, pool: PgPool) -> Result<(), JWTCheckError> {
    if jwt.is_empty() {
        eprintln!("No JWT token provided.");
        return Err(JWTCheckError::NoTokenProvided);
    }
    let user_id = match get_user_id_from_jwt(&jwt) {
        Some(id) => id,
        None => {
            eprintln!("Failed to decode JWT token.");
            return Err(JWTCheckError::InvalidToken);
        }
    };

    let parsed_user_id: usize = match user_id.parse() {
        Ok(id) => id,
        Err(_) => {
            eprintln!("Failed to parse user ID from JWT.");
            return Err(JWTCheckError::InvalidToken);
        }
    };

    let query_view: DoesUserExistByIdQueryView =
        DoesUserExistByIdQueryView::new(parsed_user_id as u64);

    let result = does_user_exist_by_id_query(query_view, pool).await;

    let exist = match result {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Database query error: {}", e);
            return Err(JWTCheckError::DatabaseError);
        }
    };

    if exist {
        let timeout: usize = match get_timeout_from_jwt(&jwt) {
            Some(t) => t,
            None => {
                eprintln!("Failed to retrieve timeout from JWT.");
                return Err(JWTCheckError::InvalidToken);
            }
        };

        match verify_jwt_timeout(timeout) {
            Ok(true) => Ok(()),
            Ok(false) => {
                eprintln!("JWT token is expired.");
                Err(JWTCheckError::ExpiredToken)
            }
            Err(e) => {
                eprintln!("Error verifying JWT timeout: {}", e);
                Err(JWTCheckError::InvalidToken)
            }
        }
    } else {
        eprintln!("User does not exist with ID: {}", user_id);
        return Err(JWTCheckError::UnknownUser);
    }
}
