use super::decode_jwt::decode_jwt;

/**
 * This function extracts the user ID from a JWT token.
 * It takes a JWT token as input and returns the user ID as a string.
 * If the decoding fails, it returns None.
 * # Arguments
 * * `jwt` - A string slice that holds the JWT token from which to extract the user ID.
 * # Returns
 * * `Option<String>` - An option that contains the user ID as a string if successful,
 *   or None if decoding fails.
 */
pub fn get_user_id_from_jwt(jwt: &str) -> Option<String> {
    match decode_jwt(jwt) {
        Ok(claims) => Some(claims.get_user_id().to_string()),
        Err(_) => None,
    }
}
