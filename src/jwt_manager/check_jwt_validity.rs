use crate::database::utils::does_user_exist_by_id;
use crate::jwt_manager::get_timeout_from_jwt;
use crate::jwt_manager::get_user_id_from_jwt;
use crate::jwt_manager::verify_jwt_timeout;

/**
 * This module provides functionality to check the validity of a JWT token.
 * It verifies if the token is provided, checks if the user exists,
 * and validates the token's expiration.
 *
 * # Errors
 * Returns an error if:
 * - No token is provided
 * - The token is expired
 * - The token is invalid
 * - The user does not exist in the database
 */
pub enum JWTCheckError {
    DatabaseError,
    NoTokenProvided,
    ExpiredToken,
    InvalidToken,
    UnknownUser,
}

/**
 * Checks the validity of a JWT token.
 *
 * # Arguments
 * - `jwt`: A string slice that holds the JWT token to be checked.
 *
 * # Returns
 * - `Ok(())` if the token is valid and the user exists.
 * - `Err(JWTCheckError)` if the token is invalid, expired, or the user does not exist.
 */
pub async fn check_jwt_validity(jwt: &str) -> Result<(), JWTCheckError> {
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

    match does_user_exist_by_id(parsed_user_id).await {
        true => {
            let timeout: usize = match get_timeout_from_jwt(&jwt) {
            Some(t) => t,
            None => {
                eprintln!("Failed to retrieve timeout from JWT.");
                return Err(JWTCheckError::InvalidToken);
            }
            };

            match verify_jwt_timeout(timeout) {
                Ok(true) => {
                    Ok(())
                }
                Ok(false) => {
                    eprintln!("JWT token is expired.");
                    Err(JWTCheckError::ExpiredToken)
                }
                Err(e) => {
                    eprintln!("Error verifying JWT timeout: {}", e);
                    Err(JWTCheckError::InvalidToken)
                }
            }
        }
        false => {
            eprintln!("User does not exist with ID: {}", user_id);
            return Err(JWTCheckError::UnknownUser);
        }
    }
}
