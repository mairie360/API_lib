use argon2::{password_hash::PasswordHasher, Argon2};

use super::error::PasswordError;

/// Hashes `password` with argon2id and a unique random salt.
///
/// The result is a self-describing PHC string (`$argon2id$v=19$m=...,t=...,p=...$<salt>$<hash>`):
/// the parameters and the salt are encoded in it, so [`super::verify_password`] needs nothing
/// else to check it later.
///
/// # Errors
///
/// Returns [`PasswordError::HashFailed`] if argon2id hashing fails (internal error of the
/// `argon2` crate, e.g. the OS random generator failing to produce the salt).
pub fn hash_password(password: &str) -> Result<String, PasswordError> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|_| PasswordError::HashFailed)
}
