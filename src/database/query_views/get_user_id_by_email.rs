use crate::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Finds the active account matching an e-mail verified by an external identity provider
/// (Keycloak) and returns its `users.id`.
///
/// The match ignores case, since Keycloak lowercases e-mails while accounts created in Core
/// keep the case they were typed with; an exact match wins if several accounts only differ by
/// case (the `users.email` constraint is case-sensitive). Archived accounts are excluded: the
/// query then returns no row (`DbError::NotFound`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GetUserIdByEmailQueryView {
    params: Vec<QueryParam>,
}

impl GetUserIdByEmailQueryView {
    #[must_use]
    pub fn new(email: &str) -> Self {
        Self {
            params: vec![QueryParam::Text(email.to_string())],
        }
    }

    #[must_use]
    pub fn get_email(&self) -> &str {
        self.params[0].as_text()
    }
}

impl ApiRequestDto for GetUserIdByEmailQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT id FROM users \
         WHERE lower(email) = lower($1) AND NOT COALESCE(is_archived, false) \
         ORDER BY (email = $1) DESC, id \
         LIMIT 1"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetUserIdByEmailQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetUserIdByEmailQueryView: email = {}", self.get_email())
    }
}
