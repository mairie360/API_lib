use serde::Deserialize;

use crate::database::db_interface::QueryParam;

/// Page size when the request gives no `limit`.
pub const DEFAULT_PAGE_SIZE: u32 = 50;

/// Largest page a request can get: a bigger `limit` is lowered to it.
pub const MAX_PAGE_SIZE: u32 = 100;

/// Largest `offset` accepted: deeper pages cost Postgres a scan of every skipped row, use a
/// filter or a cursor instead. A bigger `offset` is lowered to it.
pub const MAX_OFFSET: u32 = 10_000;

/// `?limit=&offset=` of a list route, extracted with `web::Query<PageParams>`.
///
/// Out-of-range values are clamped rather than refused, so an existing client asking for
/// everything gets the first [`MAX_PAGE_SIZE`] rows and `has_more = true`.
///
/// ```ignore
/// // view: "... ORDER BY created_at DESC, id DESC LIMIT $2 OFFSET $3"
/// params: vec![owner.into(), page.sql_limit(), page.sql_offset()],
/// // endpoint:
/// let rows: Vec<Project> = smart_db.fetch_all(&view).await?;
/// HttpResponse::Ok().json(Page::from_overfetch(rows, &page))
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub struct PageParams {
    /// Rows per page, `1..=MAX_PAGE_SIZE`, default [`DEFAULT_PAGE_SIZE`].
    pub limit: Option<u32>,
    /// Rows to skip, `0..=MAX_OFFSET`, default `0`.
    pub offset: Option<u32>,
}

impl PageParams {
    /// The page size actually served.
    #[must_use]
    pub fn limit(&self) -> u32 {
        self.limit
            .unwrap_or(DEFAULT_PAGE_SIZE)
            .clamp(1, MAX_PAGE_SIZE)
    }

    /// The offset actually applied.
    #[must_use]
    pub fn offset(&self) -> u32 {
        self.offset.unwrap_or(0).min(MAX_OFFSET)
    }

    /// `LIMIT` to bind: one row more than the page, to know whether another page exists.
    #[must_use]
    pub fn sql_limit(&self) -> QueryParam {
        QueryParam::I64(i64::from(self.limit()) + 1)
    }

    /// `OFFSET` to bind.
    #[must_use]
    pub fn sql_offset(&self) -> QueryParam {
        QueryParam::I64(i64::from(self.offset()))
    }
}
