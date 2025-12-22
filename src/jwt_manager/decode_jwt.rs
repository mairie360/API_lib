use super::get_jwt_secret::get_jwt_secret;
use super::jwt_claims::Claims;
use jsonwebtoken::{decode, DecodingKey, Validation};

/**
 * This function decode a Json Web Tocken
 * It takes a JWT token as input and returns the decoded claims.
 * If the decoding fails, it returns an error.
 * # Arguments
 * * `token` - A string slice that holds the JWT token to be decoded.
 * # Returns
 * * `Result<Claims, jsonwebtoken::errors::Error>` - A result that contains
 *   the decoded claims if successful, or an error if decoding fails.
 */
pub fn decode_jwt(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let secret: Vec<u8> = get_jwt_secret().map_err(|_e| {
        jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::InvalidKeyFormat)
    })?;
    let validation = Validation::default();
    let token_data = decode::<Claims>(token, &DecodingKey::from_secret(&secret), &validation)?;
    Ok(token_data.claims)
}
