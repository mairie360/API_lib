use super::get_jwt_secret::get_jwt_secret;
use super::jwt_claims::Claims;
use jsonwebtoken::{decode, DecodingKey, Validation};

pub fn decode_jwt(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let secret: Vec<u8> = get_jwt_secret().map_err(|_e| {
        jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::InvalidKeyFormat)
    })?;
    let validation = Validation::default();
    let token_data = decode::<Claims>(token, &DecodingKey::from_secret(&secret), &validation)?;
    Ok(token_data.claims)
}
