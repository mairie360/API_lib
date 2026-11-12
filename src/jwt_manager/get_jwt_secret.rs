use crate::env_manager::get_env_var;

/// Lit le secret de signature des JWT depuis `JWT_SECRET`.
///
/// # Errors
///
/// Returns `ErrorKind::MissingRequiredClaim` when `JWT_SECRET` is unset or blank.
pub fn get_jwt_secret() -> Result<Vec<u8>, jsonwebtoken::errors::ErrorKind> {
    // A blank secret is no secret (MAIR-680): with `JWT_PUBLIC_KEY` set the configuration is valid
    // without `JWT_SECRET`, and an empty HMAC key would let anybody sign HS256 tokens.
    get_env_var("JWT_SECRET")
        .filter(|secret| !secret.trim().is_empty())
        .map(String::into_bytes)
        .ok_or_else(|| {
            jsonwebtoken::errors::ErrorKind::MissingRequiredClaim(
                "JWT_SECRET environment variable not set".to_string(),
            )
        })
}
