use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};

use super::error::PasswordError;

/// Hache `password` en argon2id avec un sel aléatoire unique.
///
/// Le résultat est une chaîne PHC (`$argon2id$v=19$m=...,t=...,p=...$<sel>$<empreinte>`)
/// auto-descriptive : les paramètres et le sel sont encodés dedans, donc [`super::verify_password`]
/// n'a besoin d'aucune autre information pour la vérifier plus tard.
///
/// # Errors
///
/// Renvoie [`PasswordError::HashFailed`] si le hachage argon2id échoue (erreur interne de la
/// crate `argon2`, par exemple un défaut de génération aléatoire du sel).
pub fn hash_password(password: &str) -> Result<String, PasswordError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| PasswordError::HashFailed)
}
