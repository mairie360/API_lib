//! Aggregated usage telemetry (MAIR-501, epic MAIR-284).
//!
//! What is used, how much and by how many agents, never who does what.
//!
//! - [`ALLOWED_SPAN_ATTRIBUTES`] / [`is_allowed_span_attribute`]: the only attributes a trace may
//!   carry (route template, method, status, duration). The instance's OpenTelemetry Collector
//!   applies the same list (Devops/Deploiment); an API that adds span attributes checks them here.
//! - [`UsageLedger`]: counts per service, operation (route template), method, status and period.
//!   Distinct users are counted from a hash of the user id with a salt drawn at random for each
//!   period, kept in memory only, never logged, stored or exported, and dropped when the period
//!   closes together with the hashes. Only counts leave the instance, and a count under the
//!   threshold `k` is not exported (it would point at a person in a small mairie).
//! - [`usage_middleware`] records every request into the ledger (`app_data`), [`usage_metrics`]
//!   serves the closed periods in the Prometheus text format for the collector to scrape.
//!
//! Behind the `usage` feature.

mod allowlist;
mod ledger;
mod middleware;

pub use allowlist::{is_allowed_span_attribute, ALLOWED_SPAN_ATTRIBUTES};
pub use ledger::{
    UsageEntry, UsageLedger, DEFAULT_PERIOD, DEFAULT_RETAINED_PERIODS, DEFAULT_THRESHOLD,
};
pub use middleware::{usage_metrics, usage_middleware, USAGE_METRICS_PATH};
