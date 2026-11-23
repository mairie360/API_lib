use crate::database::db_interface::{id_to_sql, ApiRequestDto, QueryParam};
use std::fmt::Display;
use uuid::Uuid;

/// State of a session in the database (MAIR-772).
///
/// `session_state()` of Database answers one row for a
/// session of that user (active = not revoked, not expired, account neither archived nor suspended) and
/// none for an unknown session. The JWT middleware reads it when `session:<sid>` is missing from Redis.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionStateQueryView {
    params: Vec<QueryParam>,
}

/// Row of [`SessionStateQueryView`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct SessionStateQueryResultView {
    pub active: bool,
    pub ip: Option<String>,
    pub dpop_jkt: Option<String>,
}

impl SessionStateQueryView {
    #[must_use]
    pub fn new(session_id: Uuid, user_id: u64) -> Self {
        Self {
            params: vec![
                QueryParam::Uuid(session_id),
                QueryParam::I32(id_to_sql(user_id)),
            ],
        }
    }
}

impl ApiRequestDto for SessionStateQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (SELECT active, host(ip_address) AS ip, dpop_jkt \
         FROM session_state($1, $2)) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }

    // Never cached by `SmartDatabase`: `session:<sid>` is the cache.
    fn cache_ttl(&self) -> u64 {
        0
    }
}

impl Display for SessionStateQueryView {
    // The session id identifies a credential: never printed.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SessionStateQueryView")
    }
}
