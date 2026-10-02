use serde::{Deserialize, Serialize};

use super::page_params::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE};
use crate::database::{db_interface::QueryParam, SqlId};

/// `?before=&limit=` of an append-only list read from the newest entry (chat messages):
/// stable while new rows arrive, unlike an offset, and as cheap for the 1000th page as for the
/// first.
///
/// ```ignore
/// // view: "... WHERE chat_id = $1 AND ($2::INT4 IS NULL OR id < $2) ORDER BY id DESC LIMIT $3"
/// params: vec![chat.into(), cursor.sql_before(), cursor.sql_limit()],
/// // endpoint:
/// HttpResponse::Ok().json(CursorPage::from_overfetch(rows, &cursor, |m: &Message| m.id))
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub struct CursorParams {
    /// Only rows with an id strictly lower than this one; absent = from the newest row.
    pub before: Option<SqlId>,
    /// Rows per page, `1..=MAX_PAGE_SIZE`, default [`DEFAULT_PAGE_SIZE`].
    pub limit: Option<u32>,
}

impl CursorParams {
    /// The page size actually served.
    #[must_use]
    pub fn limit(&self) -> u32 {
        self.limit
            .unwrap_or(DEFAULT_PAGE_SIZE)
            .clamp(1, MAX_PAGE_SIZE)
    }

    /// The cursor to bind (`NULL` for the first page).
    #[must_use]
    pub fn sql_before(&self) -> QueryParam {
        QueryParam::OptionI32(self.before.map(SqlId::get))
    }

    /// `LIMIT` to bind: one row more than the page, to know whether another page exists.
    #[must_use]
    pub fn sql_limit(&self) -> QueryParam {
        QueryParam::I64(i64::from(self.limit()) + 1)
    }
}

/// One page of a cursor-paginated list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CursorPage<T> {
    /// At most `limit` rows, newest first.
    pub items: Vec<T>,
    /// Value of `before` for the next (older) page, `null` on the last page.
    pub next_before: Option<i32>,
}

impl<T> CursorPage<T> {
    /// Builds the page from rows fetched with [`CursorParams::sql_limit`] (one row too many);
    /// `id_of` gives the cursor value of a row.
    #[must_use]
    pub fn from_overfetch(
        mut rows: Vec<T>,
        params: &CursorParams,
        id_of: impl Fn(&T) -> i32,
    ) -> Self {
        let page_len = usize::try_from(params.limit()).unwrap_or(usize::MAX);
        let has_more = rows.len() > page_len;
        rows.truncate(page_len);
        let next_before = if has_more {
            rows.last().map(id_of)
        } else {
            None
        };
        Self {
            items: rows,
            next_before,
        }
    }
}
