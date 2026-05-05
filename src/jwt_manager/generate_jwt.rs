use super::get_jwt_secret::get_jwt_secret;
use super::get_jwt_timeout::get_jwt_timeout;
use super::jwt_claims::Claims;
use jsonwebtoken::{encode, EncodingKey, Header};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn generate_jwt(user_id_str: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let secret: Vec<u8> = get_jwt_secret()?;
    let timeout = get_jwt_timeout()?;

    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize
        + timeout; // Token valid for the configured JWT timeout duration
    let claims = Claims::new(user_id_str.to_owned(), expiration);
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&secret),
    )?;
    Ok(token)
}
