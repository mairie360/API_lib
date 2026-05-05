use crate::env_manager::get_env_var;

pub fn get_jwt_secret() -> Result<Vec<u8>, jsonwebtoken::errors::ErrorKind> {
    match get_env_var("JWT_SECRET") {
        Some(secret) => Ok(secret.into_bytes()),
        None => Err(jsonwebtoken::errors::ErrorKind::MissingRequiredClaim(
            "JWT_SECRET environment variable not set".to_string(),
        )),
    }
}
