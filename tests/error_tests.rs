//! `ApiLibError` status and kind mapping, and `log_error` (MAIR-421). Every error answers
//! `text/plain` (MAIR-262).

use actix_web::{
    http::{header::CONTENT_TYPE, StatusCode},
    ResponseError,
};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::database::InvalidId;
use mairie360_api_lib::error::{log_error, ApiLibError, ErrorKind};
use mairie360_api_lib::jwt_manager::error::JWTCheckError;
use mairie360_api_lib::keycloak::KeycloakError;
use mairie360_api_lib::password::error::PasswordError;
use mairie360_api_lib::redis::error::RedisError;
use mairie360_api_lib::validation::ValidationError;

fn cases() -> Vec<(ApiLibError, StatusCode, ErrorKind)> {
    vec![
        (
            DbError::NotFound.into(),
            StatusCode::NOT_FOUND,
            ErrorKind::NotFound,
        ),
        (
            DbError::UniqueViolation("users_email_key".into()).into(),
            StatusCode::CONFLICT,
            ErrorKind::Conflict,
        ),
        (
            DbError::ForeignKeyViolation("fk_project".into()).into(),
            StatusCode::BAD_REQUEST,
            ErrorKind::InvalidReference,
        ),
        (
            DbError::MappingError("missing field".into()).into(),
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorKind::Internal,
        ),
        (
            DbError::Sqlx(sqlx::Error::PoolTimedOut).into(),
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorKind::Internal,
        ),
        (
            RedisError::Pool("down".into()).into(),
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorKind::Internal,
        ),
        (
            JWTCheckError::ExpiredToken.into(),
            StatusCode::UNAUTHORIZED,
            ErrorKind::Client,
        ),
        (
            serde_json::from_str::<u8>("x").unwrap_err().into(),
            StatusCode::BAD_REQUEST,
            ErrorKind::Client,
        ),
    ]
}

#[test]
fn status_code_matches_the_response_and_the_kind() {
    for (error, status, kind) in cases() {
        assert_eq!(error.status_code(), status, "{error:?}");
        assert_eq!(error.error_response().status(), status, "{error:?}");
        assert_eq!(error.kind(), kind, "{error:?}");
        assert_eq!(
            kind.is_client_error(),
            status.is_client_error(),
            "{error:?}"
        );
    }
}

#[test]
fn log_error_gives_the_error_back() {
    for (error, status, kind) in cases() {
        let logged = log_error("test operation", error);
        assert_eq!(logged.status_code(), status);
        assert_eq!(logged.kind(), kind);
    }
}

/// MAIR-262: the APIs document every error body of the lib as `text/plain`, but they were sent
/// without a content type. `JwtMiddleware` returns its refusals as errors, so the APIs'
/// `DefaultHeaders` could not add it: the lib must.
#[test]
fn every_error_answers_text_plain() {
    let mut errors: Vec<Box<dyn ResponseError>> = cases()
        .into_iter()
        .map(|(error, _, _)| Box::new(error) as Box<dyn ResponseError>)
        .collect();
    errors.extend(
        [
            JWTCheckError::DatabaseError,
            JWTCheckError::NoTokenProvided,
            JWTCheckError::ExpiredToken,
            JWTCheckError::InvalidToken,
            JWTCheckError::UnknownUser,
            JWTCheckError::RevokedToken,
            JWTCheckError::RevocationCheckUnavailable,
            JWTCheckError::EmailNotVerified,
            JWTCheckError::IdentityProviderUnavailable,
        ]
        .map(|error| Box::new(error) as Box<dyn ResponseError>),
    );
    errors.extend(
        [
            KeycloakError::InvalidToken,
            KeycloakError::ExpiredToken,
            KeycloakError::EmailNotVerified,
            KeycloakError::Unavailable,
        ]
        .map(|error| Box::new(error) as Box<dyn ResponseError>),
    );
    errors.push(Box::new(PasswordError::HashFailed));
    errors.push(Box::new(PasswordError::InvalidHash));
    errors.push(Box::new(ValidationError::new("name", "must not be blank")));
    errors.push(Box::new(InvalidId));

    for error in errors {
        let response = error.error_response();
        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .map(|v| v.to_str().unwrap()),
            Some("text/plain; charset=utf-8"),
            "{error}"
        );
    }
}
