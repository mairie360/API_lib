use crate::database::query_views::IsUserActiveByIdQueryView;
use crate::database::query_views::{SessionStateQueryResultView, SessionStateQueryView};
use crate::jwt_manager::decode_jwt::decode_jwt;
use crate::jwt_manager::error::JWTCheckError;
use crate::jwt_manager::session_revocation::is_session_revoked;
use crate::jwt_manager::session_state::{cache_active_session, read_session_state, SessionStatus};
use crate::jwt_manager::validate_jwt_config::{is_session_required, REQUIRE_SESSION_ENV};
use crate::smart_db::SmartDatabase;
use std::time::{SystemTime, UNIX_EPOCH};

/// Checks that a JWT signed with `JWT_SECRET` is present, valid, not expired and designates an
/// existing, non-archived account.
///
/// Only concerns the historical tokens issued by `Core_API`: to accept the `EdDSA` session tokens as
/// well, use [`super::authenticate_token`].
///
/// # Errors
///
/// - [`JWTCheckError::NoTokenProvided`] si le jeton est vide ;
/// - [`JWTCheckError::ExpiredToken`] si le jeton est expiré ;
/// - [`JWTCheckError::InvalidToken`] if the token is unreadable, its `user_id` is not an integer,
///   or it has no `sid` while `JWT_REQUIRE_SESSION` is enabled;
/// - [`JWTCheckError::RevokedToken`] if the token's session (`sid` claim) was revoked;
/// - [`JWTCheckError::RevocationCheckUnavailable`] if the token has a `sid` and Redis cannot be
///   queried (fail closed);
/// - [`JWTCheckError::DatabaseError`] si la vérification en base échoue ;
/// - [`JWTCheckError::UnknownUser`] if the account does not exist or is archived.
pub async fn check_jwt_validity(
    jwt: &str,
    db_interface: &SmartDatabase,
) -> Result<(), JWTCheckError> {
    if jwt.is_empty() {
        tracing::debug!("No JWT token provided.");
        return Err(JWTCheckError::NoTokenProvided);
    }
    resolve_legacy_user(jwt, db_interface).await.map(|_| ())
}

/// Same checks as [`check_jwt_validity`] on a non-empty token, returning the `users.id` the
/// token designates.
pub(super) async fn resolve_legacy_user(
    jwt: &str,
    db_interface: &SmartDatabase,
) -> Result<u64, JWTCheckError> {
    // 1. Décodage et distinction de l'expiration vs token invalide
    let claims = decode_jwt(jwt).map_err(|err| {
        tracing::warn!(error = ?err, "JWT decode error");
        if matches!(
            err.kind(),
            jsonwebtoken::errors::ErrorKind::ExpiredSignature
        ) {
            JWTCheckError::ExpiredToken
        } else {
            JWTCheckError::InvalidToken
        }
    })?;

    // 2. Extraction et parsing de l'ID utilisateur
    let user_id_str = claims.user_id();
    let parsed_user_id: u64 = user_id_str.parse().map_err(|_| {
        tracing::warn!("Failed to parse user ID from JWT claims.");
        JWTCheckError::InvalidToken
    })?;

    // 2b. Revocation list: only tokens bound to a session can be revoked. When Redis cannot be
    // checked the token is refused (fail closed) with 503, not 401: the token itself is fine, the
    // client should retry rather than drop its session. Tokens without a session are refused
    // outright when `JWT_REQUIRE_SESSION` is enabled.
    if claims.session_id().is_none() && is_session_required() {
        tracing::info!("JWT without session (`sid`) refused: {REQUIRE_SESSION_ENV} is enabled.");
        return Err(JWTCheckError::InvalidToken);
    }
    if let Some(session_id) = claims.session_id() {
        if let Some(user_id) = check_session(
            jwt,
            session_id,
            parsed_user_id,
            claims.expiration(),
            db_interface,
        )
        .await?
        {
            return Ok(user_id);
        }
    }

    // 3. The account must exist and not be archived.
    let query_view = IsUserActiveByIdQueryView::new(parsed_user_id);
    let exist = db_interface
        .fetch_scalar::<bool, _>(&query_view)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Database query error");
            JWTCheckError::DatabaseError
        })?;

    if !exist {
        tracing::info!(user_id = %user_id_str, "No active account with this ID");
        return Err(JWTCheckError::UnknownUser);
    }

    Ok(parsed_user_id)
}

/// Checks the session of a token carrying a `sid` (MAIR-772). `Ok(Some(user_id))` accepts the token,
/// `Ok(None)` leaves the decision to the account check (session unknown to the database: tokens signed
/// outside Core's sign-in, such as test fixtures).
///
/// 1. `session:<sid>` in Redis: a revoked state refuses, another user's state or a hash that is not the
///    token's refuses (`ReplacedToken`), otherwise the token passes without any database query.
/// 2. Key missing: `revoked:<sid>` refuses (Core versions that only write the revocation list), then
///    `session_state()` decides; an active session is written back with `SET NX` (no hash).
/// 3. Redis unavailable: the database decides alone, nothing is written. A session the database does not
///    know is then refused with `503` (its revocation cannot be checked: fail closed).
async fn check_session(
    jwt: &str,
    session_id: &str,
    user_id: u64,
    expiration: usize,
    db_interface: &SmartDatabase,
) -> Result<Option<u64>, JWTCheckError> {
    let redis = db_interface.get_redis();
    let redis_up = match read_session_state(&redis, session_id).await {
        Ok(Some(state)) => {
            if state.state == SessionStatus::Revoked {
                return Err(JWTCheckError::RevokedToken);
            }
            if state.user_id != user_id {
                tracing::warn!("Session state of another account: token refused.");
                return Err(JWTCheckError::InvalidToken);
            }
            if !state.accepts(jwt) {
                return Err(JWTCheckError::ReplacedToken);
            }
            return Ok(Some(user_id));
        }
        Ok(None) => match is_session_revoked(&redis, session_id).await {
            Ok(true) => return Err(JWTCheckError::RevokedToken),
            Ok(false) => true,
            Err(e) => {
                tracing::warn!(error = %e, "Revocation lookup error, checking the database");
                false
            }
        },
        Err(e) => {
            tracing::warn!(error = %e, "Session state lookup error, checking the database");
            false
        }
    };

    let Ok(session_uuid) = uuid::Uuid::parse_str(session_id) else {
        return unknown_session(redis_up);
    };
    let rows: Vec<SessionStateQueryResultView> = match db_interface
        .fetch_all(&SessionStateQueryView::new(session_uuid, user_id))
        .await
    {
        Ok(rows) => rows,
        // A database older than v4.0.0 has no `session_state()`: the previous checks apply.
        Err(e) if is_undefined_function(&e) => {
            tracing::warn!("session_state() missing from the database: account check only.");
            return unknown_session(redis_up);
        }
        Err(e) => {
            tracing::error!(error = %e, "Session state query error");
            return Err(JWTCheckError::DatabaseError);
        }
    };
    let Some(row) = rows.into_iter().next() else {
        return unknown_session(redis_up);
    };
    if !row.active {
        // An archived account keeps its historical answer (`UnknownUser`); any other inactive session
        // (revoked, expired, suspended account) is a revoked token.
        let account_active = db_interface
            .fetch_scalar::<bool, _>(&IsUserActiveByIdQueryView::new(user_id))
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Database query error");
                JWTCheckError::DatabaseError
            })?;
        return Err(if account_active {
            JWTCheckError::RevokedToken
        } else {
            JWTCheckError::UnknownUser
        });
    }
    if redis_up {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());
        let ttl = u64::try_from(expiration)
            .unwrap_or(u64::MAX)
            .saturating_sub(now);
        if ttl > 0 {
            if let Err(e) =
                cache_active_session(&redis, session_id, user_id, row.ip.as_deref(), ttl).await
            {
                tracing::warn!(error = %e, "Session state write error");
            }
        }
    }
    Ok(Some(user_id))
}

const fn unknown_session(redis_up: bool) -> Result<Option<u64>, JWTCheckError> {
    if redis_up {
        Ok(None)
    } else {
        Err(JWTCheckError::RevocationCheckUnavailable)
    }
}

/// SQLSTATE `42883` (undefined function).
fn is_undefined_function(error: &crate::error::ApiLibError) -> bool {
    matches!(
        error,
        crate::error::ApiLibError::Database(crate::database::error::DbError::Sqlx(
            sqlx::Error::Database(db)
        )) if db.code().as_deref() == Some("42883")
    )
}
