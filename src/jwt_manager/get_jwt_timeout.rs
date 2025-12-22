use crate::env_manager::get_env_var;

/**
 * This function retrieves the JWT timeout from the environment variable.
 * It returns the timeout as a `usize` if the environment variable is set and valid.
 * If the environment variable is not set or is invalid, it returns an error.
 * # Returns
 * * `Result<usize, String>` - A result that contains the JWT timeout if successful,
 *   or an error message if the environment variable is not set or invalid.
 */
pub fn get_jwt_timeout() -> Result<usize, String> {
    match get_env_var("JWT_TIMEOUT") {
        Some(secret) => secret
            .parse::<usize>()
            .map_err(|_| "JWT_TIMEOUT is not a valid u16".to_string()),
        None => Err("JWT_TIMEOUT environment variable not set".to_string()),
    }
}
