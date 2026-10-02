use actix_web::{dev::Payload, FromRequest, HttpMessage, HttpRequest};
use futures_util::future::{ready, Ready};

/// An administrator authenticated by [`AdminMiddleware`](super::AdminMiddleware).
///
/// Only that middleware inserts it, after checking `is_admin()` in the database. Extracting it
/// in an admin handler makes the handler fail closed (`403 Forbidden`) when the request did not
/// go through the admin check, e.g. when a route is mounted outside the admin scope by mistake.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AdminUser {
    pub id: u64,
}

impl FromRequest for AdminUser {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(
            req.extensions().get::<Self>().copied().ok_or_else(|| {
                actix_web::error::ErrorForbidden("Forbidden: User is not an admin.")
            }),
        )
    }
}
