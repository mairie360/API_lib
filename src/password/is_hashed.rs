/// Indique si `value` est déjà une empreinte argon2id (`$argon2id$...`) plutôt qu'un mot de
/// passe encore en clair.
///
/// Sert aux APIs consommatrices à distinguer, à la connexion, un compte déjà migré (empreinte
/// stockée, à vérifier avec [`super::verify_password`]) d'un compte hérité de l'ancien schéma
/// en clair (à comparer directement puis à hacher immédiatement avec [`super::hash_password`]
/// pour migrer la ligne).
#[must_use]
pub fn is_hashed(value: &str) -> bool {
    value.starts_with("$argon2id$")
}
