use std::net::{IpAddr, SocketAddr};
use std::num::NonZeroU32;
use std::sync::Arc;

use actix_web::{
    body::BoxBody,
    dev::{ServiceRequest, ServiceResponse},
    http::header::{ContentType, RETRY_AFTER},
    middleware::Next,
    Error, HttpMessage, HttpResponse,
};
use governor::{clock::Clock, clock::DefaultClock, DefaultKeyedRateLimiter, Quota};

use crate::env_manager::get_env_var;
use crate::security::AuthenticatedUser;

/// Requests per second each caller may sustain; `0` disables the limiter.
pub const RATE_LIMIT_PER_SECOND_ENV: &str = "RATE_LIMIT_PER_SECOND";
/// Requests a caller may send at once above the sustained rate.
pub const RATE_LIMIT_BURST_ENV: &str = "RATE_LIMIT_BURST";
/// Set to `true` to key anonymous callers on the forwarded client address.
///
/// Reads the **last** hop of `X-Forwarded-For` (the one the trusted proxy in front of the API
/// appended) instead of the TCP peer; `Forwarded` and the hops on its left are ignored, since the
/// client writes them. Only behind exactly one proxy that appends the address it saw.
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
    let fallback = || NonZeroU32::new(default).unwrap_or(NonZeroU32::MIN);
    let Some(raw) = get_env_var(name) else {
        return fallback();
    };
    if let Some(value) = raw.trim().parse::<u32>().ok().and_then(NonZeroU32::new) {
        return value;
    }
    tracing::warn!(variable = name, default, "invalid value, using the default");
    fallback()
}

/// `true` when [`RATE_LIMIT_PER_SECOND_ENV`] is the integer `0`.
fn disabled_by_env() -> bool {
    get_env_var(RATE_LIMIT_PER_SECOND_ENV).is_some_and(|v| v.trim().parse::<u32>() == Ok(0))
}

impl RateLimitConfig {
    /// Reads [`RATE_LIMIT_PER_SECOND_ENV`], [`RATE_LIMIT_BURST_ENV`] and
    /// [`RATE_LIMIT_TRUST_FORWARDED_ENV`], with the defaults for unset or invalid values (a rate of
    /// `0` is invalid here: [`RateLimiter::from_env`] turns it into a disabled limiter instead).
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

/// Per-caller rate limiter, shared by every worker: build it **once** in `main`,
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
///
/// The counters live in memory, **per pod**: with `n` replicas a caller can send up to `n` times
/// the configured budget. `RATE_LIMIT_PER_SECOND=0` disables the limiter ([`Self::disabled`]).
#[derive(Clone)]
pub struct RateLimiter {
    /// `None` when the limiter is disabled.
    limiter: Option<Arc<DefaultKeyedRateLimiter<RateLimitKey>>>,
    trust_forwarded: bool,
}

impl RateLimiter {
    #[must_use]
    pub fn new(config: RateLimitConfig) -> Self {
        let quota = Quota::per_second(config.per_second).allow_burst(config.burst);
        Self {
            limiter: Some(Arc::new(DefaultKeyedRateLimiter::keyed(quota))),
            trust_forwarded: config.trust_forwarded,
        }
    }

    /// A limiter that lets every request through (load and scan stacks).
    #[must_use]
    pub const fn disabled() -> Self {
        Self {
            limiter: None,
            trust_forwarded: false,
        }
    }

    /// [`Self::new`] with [`RateLimitConfig::from_env`], or [`Self::disabled`] when
    /// [`RATE_LIMIT_PER_SECOND_ENV`] is `0`.
    #[must_use]
    pub fn from_env() -> Self {
        if disabled_by_env() {
            tracing::warn!("rate limiting disabled by RATE_LIMIT_PER_SECOND=0");
            return Self::disabled();
        }
        Self::new(RateLimitConfig::from_env())
    }

    fn key_of(&self, req: &ServiceRequest) -> RateLimitKey {
        if let Some(user) = req.extensions().get::<AuthenticatedUser>() {
            return RateLimitKey::User(user.id);
        }
        let forwarded = self
            .trust_forwarded
            .then(|| last_forwarded_hop(req))
            .flatten();
        forwarded
            .or_else(|| req.peer_addr().map(|addr| addr.ip()))
            .map_or(RateLimitKey::Unknown, RateLimitKey::Address)
    }

    /// `Ok` when the request fits the caller's budget, else the seconds to wait (at least 1).
    fn check(&self, key: &RateLimitKey) -> Result<(), u64> {
        let Some(limiter) = &self.limiter else {
            return Ok(());
        };
        if limiter.len() > MAX_TRACKED_KEYS {
            limiter.retain_recent();
        }
        limiter.check_key(key).map_err(|not_until| {
            not_until
                .wait_time_from(DefaultClock::default().now())
                .as_secs()
                .max(1)
        })
    }
}

/// The last hop of `X-Forwarded-For`: the address the trusted proxy saw, appended after whatever
/// the client sent. The hops on its left, like `Forwarded`, are client-controlled and ignored.
fn last_forwarded_hop(req: &ServiceRequest) -> Option<IpAddr> {
    req.headers()
        .get_all("x-forwarded-for")
        .next_back()
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.rsplit(',').next())
        .and_then(|hop| parse_address(hop.trim()))
}

/// `1.2.3.4`, `1.2.3.4:5678`, `[::1]:80` or `::1`.
fn parse_address(value: &str) -> Option<IpAddr> {
    value
        .parse::<IpAddr>()
        .ok()
        .or_else(|| value.parse::<SocketAddr>().ok().map(|addr| addr.ip()))
}

/// Answers `429 Too Many Requests` (with `Retry-After` and `X-RateLimit-After`) when the caller
/// exceeded its budget.
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
                .content_type(ContentType::plaintext())
                .insert_header((RETRY_AFTER, retry_after.to_string()))
                .insert_header(("X-RateLimit-After", retry_after.to_string()))
                .body(format!("Too many requests, retry in {retry_after}s."));
            Ok(req.into_response(response))
        }
    }
}
