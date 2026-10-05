use std::fmt;
use std::str::FromStr;

use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

use crate::database::db_interface::QueryParam;

/// Identifier of a row (`SERIAL` / `INT4` primary key) received in a request (MAIR-422).
///
/// Only `1..=i32::MAX` is accepted. Use it instead of `u64` for path and query ids: a value
/// Postgres cannot hold is refused before the handler runs, where `id as i32` used to wrap it
/// to another row (`4294967297` → `1`).
///
/// - in `web::Path<SqlId>` (or a tuple of them) an invalid id answers `404 Not Found`;
/// - in `web::Query` / `web::Json` it answers `400 Bad Request`.
///
/// ```ignore
/// pub async fn get_event(path: web::Path<SqlId>, ...) -> Result<HttpResponse, ApiError> {
///     let event_id = path.into_inner();
///     state.get_smart_db().fetch_one(&GetEventView::new(event_id)).await?;
/// }
/// // in the view: `params: vec![event_id.into()]` (a `QueryParam::I32`)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct SqlId(i32);

/// An identifier outside `1..=i32::MAX`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("Invalid identifier: expected an integer between 1 and 2147483647")]
pub struct InvalidId;

impl ResponseError for InvalidId {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::BadRequest().body(self.to_string())
    }
}

impl SqlId {
    /// The id, when it is in `1..=i32::MAX`.
    #[must_use]
    pub const fn new(id: i32) -> Option<Self> {
        if id > 0 {
            Some(Self(id))
        } else {
            None
        }
    }

    /// The value to bind in SQL.
    #[must_use]
    pub const fn get(self) -> i32 {
        self.0
    }

    /// The value as the `u64` the rest of the lib uses for ids (e.g. [`crate::security::AuthenticatedUser`]).
    #[must_use]
    pub fn as_u64(self) -> u64 {
        u64::from(self.0.unsigned_abs())
    }
}

impl TryFrom<u64> for SqlId {
    type Error = InvalidId;

    fn try_from(id: u64) -> Result<Self, InvalidId> {
        i32::try_from(id).ok().and_then(Self::new).ok_or(InvalidId)
    }
}

impl TryFrom<i64> for SqlId {
    type Error = InvalidId;

    fn try_from(id: i64) -> Result<Self, InvalidId> {
        i32::try_from(id).ok().and_then(Self::new).ok_or(InvalidId)
    }
}

impl TryFrom<i32> for SqlId {
    type Error = InvalidId;

    fn try_from(id: i32) -> Result<Self, InvalidId> {
        Self::new(id).ok_or(InvalidId)
    }
}

impl FromStr for SqlId {
    type Err = InvalidId;

    /// Decimal digits only (no sign, no whitespace), in `1..=i32::MAX`.
    fn from_str(s: &str) -> Result<Self, InvalidId> {
        if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err(InvalidId);
        }
        s.parse::<i32>().ok().and_then(Self::new).ok_or(InvalidId)
    }
}

impl<'de> Deserialize<'de> for SqlId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // `u64` covers path segments (parsed from text by actix) and JSON numbers; negative
        // values and non-integers already fail here.
        let id = u64::deserialize(deserializer)?;
        Self::try_from(id).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for SqlId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<SqlId> for i32 {
    fn from(id: SqlId) -> Self {
        id.0
    }
}

impl From<SqlId> for u64 {
    fn from(id: SqlId) -> Self {
        id.as_u64()
    }
}

impl From<SqlId> for QueryParam {
    fn from(id: SqlId) -> Self {
        Self::I32(id.0)
    }
}

/// Converts an API id (`u64`) to the `INT4` Postgres holds, refusing what does not fit instead
/// of saturating like [`super::db_interface::id_to_sql`].
///
/// # Errors
///
/// [`InvalidId`] (a `400`) when `id` is `0` or above `i32::MAX`.
pub fn try_id_to_sql(id: u64) -> Result<i32, InvalidId> {
    SqlId::try_from(id).map(SqlId::get)
}
