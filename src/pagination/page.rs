use serde::Serialize;

use super::PageParams;

/// One page of a list, the body of a paginated list route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Page<T> {
    /// At most `limit` rows.
    pub items: Vec<T>,
    /// Page size served (after clamping).
    pub limit: u32,
    /// Offset served (after clamping).
    pub offset: u32,
    /// Whether rows exist after this page (`offset + limit` is the next page's offset).
    pub has_more: bool,
}

impl<T> Page<T> {
    /// Builds the page from rows fetched with [`PageParams::sql_limit`] (one row too many).
    #[must_use]
    pub fn from_overfetch(mut rows: Vec<T>, params: &PageParams) -> Self {
        let limit = params.limit();
        let page_len = usize::try_from(limit).unwrap_or(usize::MAX);
        let has_more = rows.len() > page_len;
        rows.truncate(page_len);
        Self {
            items: rows,
            limit,
            offset: params.offset(),
            has_more,
        }
    }
}
