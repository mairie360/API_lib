//! `ApiLibError` status and kind mapping, and `log_error` (MAIR-421).

use actix_web::{http::StatusCode, ResponseError};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::{log_error, ApiLibError, ErrorKind};
use mairie360_api_lib::jwt_manager::error::JWTCheckError;
use mairie360_api_lib::redis::error::RedisError;

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
