use std::net::{IpAddr, SocketAddr};
use std::num::NonZeroU32;
use std::sync::Arc;

use actix_web::{
    body::BoxBody,
    dev::{ServiceRequest, ServiceResponse},
    http::header::RETRY_AFTER,
    middleware::Next,
    Error, HttpMessage, HttpResponse,
};
use governor::{clock::Clock, clock::DefaultClock, DefaultKeyedRateLimiter, Quota};

use crate::env_manager::get_env_var;
use crate::security::AuthenticatedUser;

/// Requests per second each caller may sustain.
pub const RATE_LIMIT_PER_SECOND_ENV: &str = "RATE_LIMIT_PER_SECOND";
/// Requests a caller may send at once above the sustained rate.
pub const RATE_LIMIT_BURST_ENV: &str = "RATE_LIMIT_BURST";
/// Set to `true` to key anonymous callers on the forwarded client address.
///
/// Reads `Forwarded` / `X-Forwarded-For` instead of the TCP peer. Only behind a proxy that overwrites these headers:
/// otherwise any caller picks its own key.
pub const RATE_LIMIT_TRUST_FORWARDED_ENV: &str = "RATE_LIMIT_TRUST_FORWARDED";

/// [`RATE_LIMIT_PER_SECOND_ENV`] when unset or invalid.
pub const DEFAULT_RATE_LIMIT_PER_SECOND: u32 = 20;
/// [`RATE_LIMIT_BURST_ENV`] when unset or invalid.
pub const DEFAULT_RATE_LIMIT_BURST: u32 = 50;

/// Above this many tracked callers, the ones back to a full budget are forgotten.
const MAX_TRACKED_KEYS: usize = 10_000;

/// Who a request's budget belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum RateLimitKey {
    /// Authenticated user (the middleware runs after `JwtMiddleware`).
    User(u64),
    /// Anonymous caller (public auth routes), by address.
    Address(IpAddr),
    /// No address known (test requests).
    Unknown,
}

/// Configuration of [`RateLimiter`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimitConfig {
    pub per_second: NonZeroU32,
    pub burst: NonZeroU32,
    pub trust_forwarded: bool,
}

fn positive_env(name: &str, default: u32) -> NonZeroU32 {
    get_env_var(name)
        .and_then(|v| v.trim().parse::<u32>().ok())
        .and_then(NonZeroU32::new)
        .or_else(|| NonZeroU32::new(default))
        .unwrap_or(NonZeroU32::MIN)
}

impl RateLimitConfig {
    /// Reads [`RATE_LIMIT_PER_SECOND_ENV`], [`RATE_LIMIT_BURST_ENV`] and
    /// [`RATE_LIMIT_TRUST_FORWARDED_ENV`], with the defaults for unset or invalid values.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            per_second: positive_env(RATE_LIMIT_PER_SECOND_ENV, DEFAULT_RATE_LIMIT_PER_SECOND),
            burst: positive_env(RATE_LIMIT_BURST_ENV, DEFAULT_RATE_LIMIT_BURST),
            trust_forwarded: get_env_var(RATE_LIMIT_TRUST_FORWARDED_ENV)
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("true")),
        }
    }
}

/// Per-caller rate limiter (MAIR-425), shared by every worker: build it **once** in `main`,
/// outside the `HttpServer::new` closure, and register a clone as app data.
///
/// ```ignore
/// let limiter = RateLimiter::from_env();
/// HttpServer::new(move || {
///     App::new()
///         .app_data(limiter.clone())
///         .service(
///             web::scope("/api")
///                 // `wrap` order: the last one runs first, so the token is checked before
///                 // the budget and each user gets their own.
///                 .wrap(from_fn(rate_limit_middleware))
///                 .wrap(JwtMiddleware)
///                 .configure(config),
///         )
/// })
/// ```
///
/// Authenticated requests are counted per user, anonymous ones (public auth routes) per
/// address. Behind a BFF, the address is the BFF's unless [`RATE_LIMIT_TRUST_FORWARDED_ENV`] is
/// set and the BFF forwards the client address.
#[derive(Clone)]
pub struct RateLimiter {
    limiter: Arc<DefaultKeyedRateLimiter<RateLimitKey>>,
    trust_forwarded: bool,
}

impl RateLimiter {
    #[must_use]
    pub fn new(config: RateLimitConfig) -> Self {
        let quota = Quota::per_second(config.per_second).allow_burst(config.burst);
        Self {
            limiter: Arc::new(DefaultKeyedRateLimiter::keyed(quota)),
            trust_forwarded: config.trust_forwarded,
        }
    }

    /// [`Self::new`] with [`RateLimitConfig::from_env`].
    #[must_use]
    pub fn from_env() -> Self {
        Self::new(RateLimitConfig::from_env())
    }

    fn key_of(&self, req: &ServiceRequest) -> RateLimitKey {
        if let Some(user) = req.extensions().get::<AuthenticatedUser>() {
            return RateLimitKey::User(user.id);
        }
        let forwarded = self
            .trust_forwarded
            .then(|| {
                req.connection_info()
                    .realip_remote_addr()
                    .and_then(parse_address)
            })
            .flatten();
        forwarded
            .or_else(|| req.peer_addr().map(|addr| addr.ip()))
            .map_or(RateLimitKey::Unknown, RateLimitKey::Address)
    }

    /// `Ok` when the request fits the caller's budget, else the seconds to wait (at least 1).
    fn check(&self, key: &RateLimitKey) -> Result<(), u64> {
        if self.limiter.len() > MAX_TRACKED_KEYS {
            self.limiter.retain_recent();
        }
        self.limiter.check_key(key).map_err(|not_until| {
            not_until
                .wait_time_from(DefaultClock::default().now())
                .as_secs()
                .max(1)
        })
    }
}

/// `1.2.3.4`, `1.2.3.4:5678`, `[::1]:80` or `::1`.
fn parse_address(value: &str) -> Option<IpAddr> {
    value
        .parse::<IpAddr>()
        .ok()
        .or_else(|| value.parse::<SocketAddr>().ok().map(|addr| addr.ip()))
}

/// Answers `429 Too Many Requests` (with `Retry-After`) when the caller exceeded its budget.
///
/// See [`RateLimiter`]. Without a `RateLimiter` in app data, requests pass (and an error is
/// logged once per request), so a wiring mistake never locks an API.
///
/// # Errors
///
/// Returns the error of the wrapped service.
pub async fn rate_limit_middleware(
    req: ServiceRequest,
    next: Next<BoxBody>,
) -> Result<ServiceResponse<BoxBody>, Error> {
    let Some(limiter) = req.app_data::<RateLimiter>().cloned() else {
        tracing::error!("rate_limit_middleware without a RateLimiter in app data: not limited");
        return next.call(req).await;
    };
    let key = limiter.key_of(&req);
    match limiter.check(&key) {
        Ok(()) => next.call(req).await,
        Err(retry_after) => {
            tracing::warn!(key = ?key, retry_after, "rate limit exceeded");
            let response = HttpResponse::TooManyRequests()
                .insert_header((RETRY_AFTER, retry_after.to_string()))
                .body("Too many requests, retry later");
            Ok(req.into_response(response))
        }
    }
}
