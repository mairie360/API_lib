/// The span attributes a trace may carry (MAIR-501).
///
/// What was done and how it went, never who did it nor what they sent: no user id, e-mail, name,
/// query string (`http.target`, `url.query`), body, client address or user agent.
pub const ALLOWED_SPAN_ATTRIBUTES: &[&str] = &[
    "http.request.method",
    "http.method",
    "http.route",
    "http.response.status_code",
    "http.status_code",
    "otel.name",
    "otel.kind",
    "otel.status_code",
    "service.name",
    "service.namespace",
    "deployment.environment.name",
    "duration_ms",
];

/// Whether `name` may be put on a span (see [`ALLOWED_SPAN_ATTRIBUTES`]).
#[must_use]
pub fn is_allowed_span_attribute(name: &str) -> bool {
    ALLOWED_SPAN_ATTRIBUTES.contains(&name)
}
