use tracing::Subscriber;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{EnvFilter, Layer};

use crate::env_manager::get_env_var;

/// Set to `json` for one JSON object per log line (production, log collectors); anything else
/// or unset gives the human-readable format.
pub const LOG_FORMAT_ENV: &str = "LOG_FORMAT";

/// Filter used when `RUST_LOG` is not set.
const DEFAULT_FILTER: &str = "info";

/// Hides the root span of `tracing_actix_web::TracingLogger` from the logs: the text format
/// prints the fields of the enclosing spans in front of each event, and that span holds the
/// client address and the query string (MAIR-290).
const ROOT_SPAN_DIRECTIVE: &str = "tracing_actix_web=off";

/// Log layer writing to `writer`, shared by every API so their logs look the same (MAIR-503).
///
/// Coloured text (the format of Core API), or one JSON object per line with `LOG_FORMAT=json`.
/// Filtered by `RUST_LOG` (default `info`), without the fields of the request root span.
#[must_use]
pub fn log_layer<S, W>(writer: W) -> Box<dyn Layer<S> + Send + Sync + 'static>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    W: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
{
    let base = get_env_var("RUST_LOG")
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_FILTER.to_owned());
    let filter = EnvFilter::new(format!("{base},{ROOT_SPAN_DIRECTIVE}"));
    let json = get_env_var(LOG_FORMAT_ENV).is_some_and(|v| v.trim().eq_ignore_ascii_case("json"));
    let layer = tracing_subscriber::fmt::layer()
        .with_writer(writer)
        .with_target(true);
    if json {
        layer.json().with_filter(filter).boxed()
    } else {
        layer.with_filter(filter).boxed()
    }
}
