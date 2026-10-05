//! Input validation shared by every request body and query string of the APIs (MAIR-426).
//!
//! A request view implements [`Validate`] and the handler extracts it with [`ValidatedJson`] or
//! [`ValidatedQuery`] instead of `web::Json` / `web::Query`: an invalid value is rejected with a
//! `400 Bad Request` (plain-text body naming the field) before the handler runs, so it never
//! reaches Postgres (where an over-long value or a NUL byte ends in a `500`).
//!
//! Free text is stored as typed: `<`, `>` and `&` are legitimate ("budget > 10 000 €", "->",
//! "<3"). Responses are JSON served with `X-Content-Type-Options: nosniff`, so the browser never
//! renders them as HTML; escaping is the job of the front that displays the value. Do not add a
//! markup filter to make a ZAP alert go away: tune the rule in `.zap/rules.tsv` instead.
mod checks;
pub use checks::{check_description, check_label, check_opaque, check_optional};
mod extractors;
pub use extractors::{ValidatedJson, ValidatedQuery};
mod validate;
pub use validate::Validate;
mod validation_error;
pub use validation_error::ValidationError;
