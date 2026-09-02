use crate::database::query_views::DoesUserExistByIdQueryView;
use crate::jwt_manager::decode_jwt::decode_jwt;
use crate::jwt_manager::error::JWTCheckError;
use crate::smart_db::SmartDatabase;

pub async fn check_jwt_validity(
    jwt: &str,
    db_interface: &SmartDatabase,
) -> Result<(), JWTCheckError> {
    if jwt.is_empty() {
        eprintln!("No JWT token provided.");
        return Err(JWTCheckError::NoTokenProvided);
    }

    // 1. Décodage et distinction de l'expiration vs token invalide
    let claims = decode_jwt(jwt).map_err(|err| {
        eprintln!("JWT decode error: {:?}", err);
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
        eprintln!("Failed to parse user ID from JWT claims.");
        JWTCheckError::InvalidToken
    })?;

    // 3. Vérification en base de données
    let query_view = DoesUserExistByIdQueryView::new(parsed_user_id);
    let exist = db_interface
        .fetch_scalar::<bool, _>(&query_view)
        .await
        .map_err(|e| {
            eprintln!("Database query error: {}", e);
            JWTCheckError::DatabaseError
        })?;

    if !exist {
        eprintln!("User does not exist with ID: {}", user_id_str);
        return Err(JWTCheckError::UnknownUser);
    }

    Ok(())
}
