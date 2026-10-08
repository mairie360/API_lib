//! Request logging without personal data (MAIR-290).
//!
//! Rule: a log describes a request by its type and context, never by the value it received.
//! The GDPR log marker test found the same leaks in every API, which this module removes:
//!
//! - actix's `Logger::default()` writes the request line, query string included, and the
//!   `Referer`: [`request_logger`] logs the method and the path only;
//! - `tracing_actix_web::TracingLogger` records the query string in `http.target` and the
//!   `Debug` of the error in `exception.details` (a token, a value quoted by serde):
//!   [`hide_query`] / [`restore_query`] keep the query out of the root span and
//!   [`RedactedRootSpanBuilder`] records [`describe_error`] instead (feature `tracing-actix`).

mod logger;
#[cfg(feature = "tracing-actix")]
mod tracing_span;

pub use logger::{request_logger, REQUEST_LOG_FORMAT};
#[cfg(feature = "tracing-actix")]
pub use tracing_span::{describe_error, hide_query, restore_query, RedactedRootSpanBuilder};
