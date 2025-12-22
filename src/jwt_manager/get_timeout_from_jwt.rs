use super::decode_jwt::decode_jwt;

/**
 * This function retrieves the expiration time from a JWT token.
 * It takes a JWT token as input and returns the expiration time in seconds.
 * If the decoding fails, it returns None.
 * # Arguments
 * * `jwt` - A string slice that holds the JWT token to be decoded.
 * # Returns
 * * `Option<usize>` - An optional value that contains the expiration time in seconds if successful,
 *   or None if decoding fails.
 */
pub fn get_timeout_from_jwt(jwt: &str) -> Option<usize> {
    match decode_jwt(jwt) {
        Ok(claims) => Some(claims.get_expiration()),
        Err(_) => None,
    }
}
