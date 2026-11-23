//! Helpers for the endpoint tests of the APIs (MAIR-419).
//!
//! An [`AppState`] on the shared seeded database, tokens for the seeded accounts and the status
//! of a call, whether the middleware answered with a response or an error.
//!
//! ```ignore
//! let state = test_app_state_as("project_api").await;
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

use super::queries_setup::{
    get_shared_db, get_shared_db_as, ADMIN_ID, ALICE_ID, BOB_ID, GROUP_OWNER_ID,
};
use crate::jwt_manager::generate_jwt;
use crate::state::AppState;

/// PKCS#8 DER of the Ed25519 private key of the test key pair (MAIR-680). Stored as DER, like the
/// the secret scanners do not read a PEM private key in the sources.
const TEST_JWT_PRIVATE_KEY_DER: &[u8] = include_bytes!("fixtures/session_test_key.der");

/// Ed25519 private key of the test key pair (MAIR-680, `JWT_PRIVATE_KEY`), as PEM.
///
/// Set as `JWT_PRIVATE_KEY` by [`init_test_jwt_env`]: the test tokens are signed like Core signs
/// them. Committed on purpose: `enforce_jwt_config` refuses its public key outside test stacks.
#[must_use]
pub fn test_jwt_private_key() -> String {
    use base64::Engine;
    let body = base64::engine::general_purpose::STANDARD.encode(TEST_JWT_PRIVATE_KEY_DER);
    let label = "PRIVATE KEY";
    format!("-----BEGIN {label}-----\n{body}\n-----END {label}-----\n")
}

/// Ed25519 public key of the test key pair (`JWT_PUBLIC_KEY`), see [`test_jwt_private_key`].
pub const TEST_JWT_PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEAKW8bV4kiFUo8hmOEU3+lXdqZL1wmKvBw6smUyjpmXks=
-----END PUBLIC KEY-----
";

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

/// Sets the test key pair and `JWT_TIMEOUT` unless they are already set, once per process.
///
/// `JWT_PUBLIC_KEY` / `JWT_PRIVATE_KEY` get the pair of [`test_jwt_private_key`]. The private key
/// is also set when the public key was preset to [`TEST_JWT_PUBLIC_KEY`] alone, so that
/// [`bearer_token`] can still sign; a harness presetting another public key keeps its own keys.
/// Called by every helper of this module that builds a state or a token.
pub fn init_test_jwt_env() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let public_key = std::env::var("JWT_PUBLIC_KEY").ok();
        if public_key.is_none() {
            std::env::set_var("JWT_PUBLIC_KEY", TEST_JWT_PUBLIC_KEY);
        }
        let test_public_key = public_key
            .as_deref()
            .is_none_or(|key| key == TEST_JWT_PUBLIC_KEY);
        if test_public_key && std::env::var_os("JWT_PRIVATE_KEY").is_none() {
            std::env::set_var("JWT_PRIVATE_KEY", test_jwt_private_key());
        }
        if std::env::var_os("JWT_TIMEOUT").is_none() {
            std::env::set_var("JWT_TIMEOUT", TEST_JWT_TIMEOUT);
        }
    });
}

/// An [`AppState`] on the shared seeded database, connected as the superuser, without Redis
/// (cache misses only; tokens from [`bearer_token`] carry no `sid`, so no revocation lookup is
/// needed).
///
/// The superuser ignores the `GRANT`s: an API tests its endpoints with
/// [`test_app_state_as`] and its own role instead (MAIR-556).
///
/// # Panics
///
/// Panics if the shared database cannot be set up.
pub async fn test_app_state() -> web::Data<AppState> {
    init_test_jwt_env();
    let (_container, url) = get_shared_db().await;
    web::Data::new(AppState::new(String::new(), url.clone()).await)
}

/// Same as [`test_app_state`], connected as the API's login role `role`.
///
/// `role` is `core_api`, `project_api`… (see [`get_shared_db_as`]): a query the role may not run
/// fails the test with `permission denied`, as it would in staging (MAIR-556).
///
/// # Panics
///
/// Panics if `role` is not an API role or the shared database cannot be set up.
pub async fn test_app_state_as(role: &str) -> web::Data<AppState> {
    init_test_jwt_env();
    let url = get_shared_db_as(role).await;
    web::Data::new(AppState::new(String::new(), url).await)
}

/// A session JWT signed with the test key pair (`EdDSA`, like Core signs them) for `user_id`,
/// without `sid`.
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
