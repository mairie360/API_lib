use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum QueryError {
    /// L'email est mal formé (vérification effectuée avant l'envoi à la DB).
    #[error("Invalid email format: {0}")]
    InvalidEmailFormat(String),

    /// La requête SQL est syntaxiquement incorrecte (ex: nom de table ou colonne erroné).
    #[error("SQL syntax error: {0}")]
    SyntaxError(String),

    /// Une contrainte de la base de données a été violée (ex: UNIQUE, NOT NULL).
    #[error("Database constraint violation: {0}")]
    ConstraintViolation(String),

    /// La requête n'a renvoyé aucun résultat alors qu'au moins une ligne était attendue.
    #[error("No results found for the given criteria")]
    NoResults,

    /// Échec lors de l'extraction des colonnes ou de la conversion vers les types Rust.
    #[error("Data mapping error: {0}")]
    MappingError(String),

    /// Le nombre de lignes modifiées ne correspond pas à ce qui était attendu (ex: Register).
    #[error("Unexpected number of rows affected. Expected {expected}, got {actual}")]
    AffectedRowsMismatch { expected: u64, actual: u64 },

    /// Erreur générique lors de l'exécution de la requête.
    #[error("Query execution failed: {0}")]
    ExecutionFailed(String),

    /// Mauvais ID fourni (ex: DoesUserExistById avec un ID qui n'existe pas).
    #[error("Invalid ID provided: {0}")]
    InvalidId(String),

    /// Mauvais mot de passe fourni (ex: Login avec un mot de passe incorrect).
    #[error("Invalid password provided for email: {0}")]
    InvalidPassword(String),

    /// Mauvais email fourni (ex: Login avec un email qui n'existe pas).
    #[error("Email not found: {0}")]
    EmailNotFound(String),
}

/// Permet de convertir facilement les erreurs tokio_postgres en QueryError
impl From<tokio_postgres::Error> for QueryError {
    fn from(err: tokio_postgres::Error) -> Self {
        // On peut affiner ici en analysant err.code() si besoin
        let err_str = err.to_string();
        if err_str.contains("unique constraint") {
            QueryError::ConstraintViolation(err_str)
        } else if err_str.contains("0 rows") {
            QueryError::NoResults
        } else {
            QueryError::ExecutionFailed(err_str)
        }
    }
}
