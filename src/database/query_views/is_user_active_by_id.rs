use crate::database::db_interface::{id_from_sql, id_to_sql, ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Returns whether `id` designates an existing, **non-archived** account.
///
/// This is the check every token goes through ([`crate::jwt_manager::check_jwt_validity`],
/// [`crate::jwt_manager::authenticate_token`]): an archived account must lose access at once,
/// not when its token expires (MAIR-391). Use [`super::DoesUserExistByIdQueryView`] to know
/// whether a row exists at all, archived or not.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IsUserActiveByIdQueryView {
    params: Vec<QueryParam>,
}

impl IsUserActiveByIdQueryView {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self {
            params: vec![QueryParam::I32(id_to_sql(id))],
        }
    }

    #[must_use]
    pub fn get_id(&self) -> u64 {
        id_from_sql(self.params[0].as_i32())
    }
}

impl ApiRequestDto for IsUserActiveByIdQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1 AND NOT COALESCE(is_archived, false)) \
         AS is_user_active"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for IsUserActiveByIdQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IsUserActiveByIdQueryView: id = {}", self.get_id())
    }
}
