use argon2::{
    password_hash::{phc::PasswordHash, PasswordVerifier},
    Argon2,
};

use super::error::PasswordError;

/// Checks that `password` matches the argon2id `hash` (PHC string `$argon2id$...`, as produced
/// by [`super::hash_password`]).
///
/// Only call it on a value for which [`super::is_hashed`] is true: on a password still stored in
/// clear text (accounts migrated from the old schema), parsing fails with
/// [`PasswordError::InvalidHash`] instead of comparing in clear text.
///
/// # Errors
///
/// Returns [`PasswordError::InvalidHash`] if `hash` is not a valid argon2id PHC string.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, PasswordError> {
    let parsed_hash = PasswordHash::new(hash).map_err(|_| PasswordError::InvalidHash)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}
