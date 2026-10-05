//! Bounded list parameters shared by every API (MAIR-425).
//!
//! A list route takes [`PageParams`] (`?limit=&offset=`) or, for an append-only list read from
//! the newest entry (chat messages), [`CursorParams`] (`?before=&limit=`). The view fetches one
//! row more than the page (`LIMIT` bound to `sql_limit()`) and [`Page::from_overfetch`] /
//! [`CursorPage::from_overfetch`] turn that extra row into `has_more` / `next_before`, without a
//! `COUNT(*)`.
mod cursor;
pub use cursor::{CursorPage, CursorParams};
mod page;
pub use page::Page;
mod page_params;
pub use page_params::{PageParams, DEFAULT_PAGE_SIZE, MAX_OFFSET, MAX_PAGE_SIZE};
