use argon2::{
    password_hash::{PasswordHash, PasswordVerifier},
    Argon2,
};

use super::error::PasswordError;

/// Vérifie que `password` correspond à l'empreinte argon2id `hash` (format PHC
/// `$argon2id$...`, tel que produit par [`super::hash_password`]).
///
/// N'appeler cette fonction que sur une valeur dont [`super::is_hashed`] est vrai : sur un mot
/// de passe encore en clair (comptes migrés depuis l'ancien schéma), le parsing échoue avec
/// [`PasswordError::InvalidHash`] plutôt que de comparer en clair.
///
/// # Errors
///
/// Renvoie [`PasswordError::InvalidHash`] si `hash` n'est pas une chaîne PHC argon2id valide.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, PasswordError> {
    let parsed_hash = PasswordHash::new(hash).map_err(|_| PasswordError::InvalidHash)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}
