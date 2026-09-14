use crate::env_manager::get_env_var;

/// Lit le secret de signature des JWT depuis `JWT_SECRET`.
///
/// # Errors
///
/// Renvoie `ErrorKind::MissingRequiredClaim` si `JWT_SECRET` n'est pas défini.
pub fn get_jwt_secret() -> Result<Vec<u8>, jsonwebtoken::errors::ErrorKind> {
    get_env_var("JWT_SECRET")
        .map(String::into_bytes)
        .ok_or_else(|| {
            jsonwebtoken::errors::ErrorKind::MissingRequiredClaim(
                "JWT_SECRET environment variable not set".to_string(),
            )
        })
}
