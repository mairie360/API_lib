use actix_web::HttpMessage;

use actix_web::{dev::Payload, FromRequest, HttpRequest};
use futures_util::future::{ready, Ready};

#[derive(Copy, Clone)]
pub struct AuthenticatedUser {
    pub id: u64,
}

impl FromRequest for AuthenticatedUser {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        // Comme ton Middleware a DEJA validé le token et l'a mis dans les extensions :
        if let Some(user) = req.extensions().get::<AuthenticatedUser>() {
            return ready(Ok(AuthenticatedUser { id: user.id }));
        }

        // Si on arrive ici, c'est que le middleware n'a pas fait son job
        // ou que la route n'est pas protégée
        ready(Err(actix_web::error::ErrorUnauthorized(
            "User not authenticated",
        )))
    }
}
