use crate::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Finds the active account matching an e-mail verified by an external identity provider
/// (Keycloak) and returns its `users.id`.
///
/// The match ignores case, since Keycloak lowercases e-mails while accounts created in Core
/// keep the case they were typed with. The `users.email` constraint is case-sensitive, though,
/// so several active accounts can match up to case: the match is then **refused** (no row,
/// `DbError::NotFound`) instead of picking one, otherwise whoever registers a colleague's e-mail
/// with another case would receive the colleague's Keycloak logins (MAIR-391). Archived accounts
/// are excluded and never make a match ambiguous.
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
        "SELECT (array_agg(id))[1] FROM users \
         WHERE lower(email) = lower($1) AND NOT COALESCE(is_archived, false) \
         HAVING count(*) = 1"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetUserIdByEmailQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never the e-mail itself (MAIR-290): a view is described, not its data.
        write!(f, "GetUserIdByEmailQueryView")
    }
}
