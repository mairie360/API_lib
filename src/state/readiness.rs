use std::future::Future;
use std::time::Duration;

use actix_web::HttpResponse;

/// Longest time a dependency check of [`super::AppState::readiness`] may take before it counts
/// as down. Keep the readiness probe's `timeoutSeconds` above it.
pub const READINESS_CHECK_TIMEOUT: Duration = Duration::from_secs(2);

/// Which dependencies answered the readiness checks (MAIR-423).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Readiness {
    /// `SELECT 1` succeeded.
    pub postgres: bool,
    /// A Redis round-trip succeeded.
    pub redis: bool,
}

impl Readiness {
    /// Both dependencies answered.
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        self.postgres && self.redis
    }

    /// Names of the dependencies that did not answer (`postgres`, `redis`).
    #[must_use]
    pub fn down(&self) -> Vec<&'static str> {
        [("postgres", self.postgres), ("redis", self.redis)]
            .into_iter()
            .filter_map(|(name, up)| (!up).then_some(name))
            .collect()
    }

    /// The readiness probe's answer: `200 ready`, or `503 not ready: postgres, redis` naming the
    /// dependencies that are down. Every API serves it on `GET /ready`; `GET /health` (liveness)
    /// checks nothing, so an outage of a dependency never makes Kubernetes restart the pods.
    #[must_use]
    pub fn to_response(&self) -> HttpResponse {
        if self.is_ready() {
            HttpResponse::Ok().body("ready")
        } else {
            HttpResponse::ServiceUnavailable()
                .body(format!("not ready: {}", self.down().join(", ")))
        }
    }
}

/// `true` when `check` succeeds within [`READINESS_CHECK_TIMEOUT`].
pub(super) async fn within_timeout<E>(check: impl Future<Output = Result<(), E>>) -> bool {
    matches!(
        tokio::time::timeout(READINESS_CHECK_TIMEOUT, check).await,
        Ok(Ok(()))
    )
}
