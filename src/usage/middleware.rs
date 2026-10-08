use std::time::{Instant, SystemTime};

use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::middleware::Next;
use actix_web::{web, Error, HttpMessage, HttpResponse};

use super::UsageLedger;
use crate::security::AuthenticatedUser;

/// Path the collector scrapes. Not exposed by the ingress (the APIs are never exposed); the
/// chart's network policy only lets the collector reach it.
pub const USAGE_METRICS_PATH: &str = "/internal/usage";

/// Records each request into the `web::Data<UsageLedger>` of the app (MAIR-501).
///
/// Route template (never the path with its values), method, status, duration and the
/// authenticated user's id, which the ledger only keeps as a salted hash until the period
/// closes. Without the ledger in `app_data` it does nothing. Wrap it outside the auth middlewares' scope so that refused requests count
/// too; the user is known when the request went through `JwtMiddleware`.
///
/// # Errors
///
/// The errors of the inner services.
pub async fn usage_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let ledger = req.app_data::<web::Data<UsageLedger>>().cloned();
    let started = Instant::now();
    let response = next.call(req).await?;
    if let Some(ledger) = ledger {
        let request = response.request();
        let operation = request
            .match_pattern()
            .unwrap_or_else(|| "unmatched".to_string());
        let user = request
            .extensions()
            .get::<AuthenticatedUser>()
            .map(|u| u.id);
        ledger.record(
            &operation,
            request.method().as_str(),
            response.status().as_u16(),
            started.elapsed(),
            user,
            SystemTime::now(),
        );
    }
    Ok(response)
}

/// `GET /internal/usage`: the closed periods of the ledger in the Prometheus text format.
pub async fn usage_metrics(ledger: web::Data<UsageLedger>) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4")
        .body(ledger.render_prometheus(SystemTime::now()))
}
