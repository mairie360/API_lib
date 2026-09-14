use jsonwebtoken::errors::ErrorKind::InvalidKeyFormat;

use crate::env_manager::get_env_var;

/// Lit la durée de validité des JWT (en secondes) depuis `JWT_TIMEOUT`.
///
/// # Errors
///
/// Renvoie `ErrorKind::MissingRequiredClaim` si `JWT_TIMEOUT` n'est pas défini et
/// `ErrorKind::InvalidKeyFormat` s'il ne s'agit pas d'un entier positif.
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
