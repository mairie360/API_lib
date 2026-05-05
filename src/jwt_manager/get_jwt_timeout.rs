use jsonwebtoken::errors::ErrorKind::InvalidKeyFormat;

use crate::env_manager::get_env_var;

pub fn get_jwt_timeout() -> Result<usize, jsonwebtoken::errors::ErrorKind> {
    match get_env_var("JWT_TIMEOUT") {
        Some(secret) => {
            let secret = secret.parse::<usize>().map_err(|_| InvalidKeyFormat)?;
            Ok(secret)
        }
        None => Err(jsonwebtoken::errors::ErrorKind::MissingRequiredClaim(
            "JWT_TIMEOUT environment variable not set".to_string(),
        )),
    }
}
