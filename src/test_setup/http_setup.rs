//! Helpers for the endpoint tests of the APIs (MAIR-419).
//!
//! An [`AppState`] on the shared seeded database, tokens for the seeded accounts and the status
//! of a call, whether the middleware answered with a response or an error.
//!
//! ```ignore
//! let state = test_app_state().await;
//! let app = test::init_service(App::new().app_data(state).service(
//!     web::scope("/api").wrap(JwtMiddleware).configure(config),
//! )).await;
//! let req = test::TestRequest::get()
//!     .uri("/api/v1/projects/1")
//!     .insert_header(authorization_for(SeededUser::GroupOwner).await)
//!     .to_request();
//! assert_eq!(status_of(&app, req).await, StatusCode::FORBIDDEN);
//! ```

use std::sync::Once;

use actix_web::dev::{Service, ServiceResponse};
use actix_web::http::header::AUTHORIZATION;
use actix_web::http::StatusCode;
use actix_web::web;

use super::queries_setup::{get_shared_db, ADMIN_ID, ALICE_ID, BOB_ID, GROUP_OWNER_ID};
use crate::jwt_manager::generate_jwt;
use crate::state::AppState;

/// `JWT_SECRET` set by [`init_test_jwt_env`] when the test binary did not set one.
pub const TEST_JWT_SECRET: &str = "test-only-jwt-secret-for-endpoint-tests";

/// `JWT_TIMEOUT` (seconds) set by [`init_test_jwt_env`] when the test binary did not set one.
pub const TEST_JWT_TIMEOUT: &str = "3600";

/// Role claim of the tokens built by [`bearer_token`]. The middlewares never read it: admin
/// rights come from the database (`IsAdminQueryView`), not from the token.
const TEST_ROLE: &str = "user";

/// The accounts seeded by [`get_shared_db`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeededUser {
    /// `alice@example.com`, active, has role 1 (admin) and owns `document` 1.
    Alice,
    /// `bob@example.com`, **archived**: every token check refuses it.
    Bob,
    /// `admin@test.com`, active, has role 1 (admin).
    Admin,
    /// `owner@test.com`, active, no role: the regular user to test refusals with.
    GroupOwner,
}

impl SeededUser {
    /// Id of the account, starting the shared database first if needed.
    ///
    /// # Panics
    ///
    /// Panics if the shared database cannot be set up.
    pub async fn id(self) -> u64 {
        get_shared_db().await;
        let cell = match self {
            Self::Alice => &ALICE_ID,
            Self::Bob => &BOB_ID,
            Self::Admin => &ADMIN_ID,
            Self::GroupOwner => &GROUP_OWNER_ID,
        };
        let id = *cell
            .get()
            .expect("seeded id missing after the shared setup");
        u64::try_from(id).expect("seeded ids are positive")
    }
}

/// Sets `JWT_SECRET` and `JWT_TIMEOUT` to test values unless they are already set, once per
/// process. Called by every helper of this module that builds a state or a token.
pub fn init_test_jwt_env() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        if std::env::var_os("JWT_SECRET").is_none() {
            std::env::set_var("JWT_SECRET", TEST_JWT_SECRET);
        }
        if std::env::var_os("JWT_TIMEOUT").is_none() {
            std::env::set_var("JWT_TIMEOUT", TEST_JWT_TIMEOUT);
        }
    });
}

/// An [`AppState`] on the shared seeded database, without Redis (cache misses only; tokens
/// from [`bearer_token`] carry no `sid`, so no revocation lookup is needed).
///
/// # Panics
///
/// Panics if the shared database cannot be set up.
pub async fn test_app_state() -> web::Data<AppState> {
    init_test_jwt_env();
    let (_container, url) = get_shared_db().await;
    web::Data::new(AppState::new(String::new(), url.clone()).await)
}

/// A historical JWT (signed with `JWT_SECRET`) for `user_id`, without `sid`.
///
/// # Panics
///
/// Panics if the token cannot be signed.
#[must_use]
pub fn bearer_token(user_id: u64) -> String {
    init_test_jwt_env();
    generate_jwt(&user_id.to_string(), TEST_ROLE).expect("failed to sign the test token")
}

/// The `Authorization: Bearer …` header for `user_id`, ready for `TestRequest::insert_header`.
#[must_use]
pub fn authorization(user_id: u64) -> (actix_web::http::header::HeaderName, String) {
    (AUTHORIZATION, format!("Bearer {}", bearer_token(user_id)))
}

/// The `Authorization` header of a seeded account (see [`authorization`]).
///
/// # Panics
///
/// Panics if the shared database cannot be set up.
pub async fn authorization_for(user: SeededUser) -> (actix_web::http::header::HeaderName, String) {
    authorization(user.id().await)
}

/// Status of `req` on `app`, whether the service answered with a response or with an error (the
/// middlewares refuse with an `Err`, which `test::call_service` turns into a panic).
pub async fn status_of<S, R, B>(app: &S, req: R) -> StatusCode
where
    S: Service<R, Response = ServiceResponse<B>, Error = actix_web::Error>,
{
    match actix_web::test::try_call_service(app, req).await {
        Ok(response) => response.status(),
        Err(error) => error.as_response_error().status_code(),
    }
}
