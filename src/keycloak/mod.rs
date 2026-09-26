//! Validation of the access tokens issued by the instance's Keycloak realm.
//!
//! During the transition from the historical password login, an API receives two kinds of
//! bearer tokens: the JWTs signed by `Core_API` with the shared `JWT_SECRET` (HMAC) and the
//! access tokens issued by Keycloak (asymmetric signature, published on the realm's JWKS
//! endpoint). [`KeycloakTokenVerifier`] handles the second kind: it checks the signature against
//! the realm keys (cached, refreshed once on key rotation), the issuer, the audience, the expiry
//! and that the token is an access token, then returns the [`KeycloakIdentity`] it asserts. The
//! `jwt_manager::authenticate_token` function and the `security` middlewares pick the right
//! path from the token's `alg` header.
//!
//! The configuration comes from the environment ([`KeycloakConfig::from_env`]); without it the
//! middlewares only accept the historical JWTs.

mod config;
pub use config::KeycloakConfig;

mod error;
pub use error::KeycloakError;

mod identity;
pub use identity::KeycloakIdentity;

mod verifier;
pub use verifier::KeycloakTokenVerifier;
