use crate::env_manager::get_env_var;

/**
 * This function retrieves the JWT secret from the environment variables.
 * It looks for the "JWT_SECRET" variable and returns its value as a byte vector.
 * If the variable is not set, it returns an error.
 * # Returns
 * * `Result<Vec<u8>, String>` - A result that contains the JWT secret as
 *   a byte vector if successful, or an error message if the variable is not set.
 */
pub fn get_jwt_secret() -> Result<Vec<u8>, String> {
    match get_env_var("JWT_SECRET") {
        Some(secret) => Ok(secret.into_bytes()),
        None => Err("JWT_SECRET environment variable not set".to_string()),
    }
}
