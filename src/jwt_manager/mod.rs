mod authenticate_token;
pub use authenticate_token::authenticate_token;

mod check_jwt_validity;
pub use check_jwt_validity::check_jwt_validity;

mod decode_jwt;
pub use decode_jwt::decode_jwt;

pub mod error;

mod generate_jwt;
pub use generate_jwt::{generate_jwt, generate_session_jwt};

mod get_jwt_from_request;
pub use get_jwt_from_request::get_jwt_from_request;

mod get_role;
pub use get_role::get_role_from_jwt;

mod get_jwt_secret;
pub use get_jwt_secret::get_jwt_secret;

mod get_jwt_timeout;
pub use get_jwt_timeout::get_jwt_timeout;

mod get_timeout_from_jwt;
pub use get_timeout_from_jwt::get_timeout_from_jwt;

mod get_user_id_from_jwt;
pub use get_user_id_from_jwt::get_user_id_from_jwt;

mod jwt_claims;
pub use jwt_claims::Claims;

mod validate_jwt_config;
pub use validate_jwt_config::{
    enforce_jwt_config, is_session_required, validate_jwt_config, JwtConfigError,
    ALLOW_WEAK_SECRET_ENV, MIN_JWT_SECRET_LEN, REQUIRE_SESSION_ENV,
};

mod session_keys;
pub use session_keys::{
    parse_public_keys, parse_signing_key, session_jwks, session_public_keys, session_signing_key,
    SessionKeyError, SessionPublicKey, SessionSigningKey, PRIVATE_KEY_ENV, PUBLIC_KEY_ENV,
    SESSION_JWT_AUDIENCE, SESSION_JWT_ISSUER,
};

mod session_revocation;
pub use session_revocation::{
    is_session_revoked, revoke_session, revoked_session_key, REVOKED_SESSION_KEY_PREFIX,
};
