use super::decode_jwt::decode_jwt;

pub fn get_timeout_from_jwt(jwt: &str) -> Option<usize> {
    match decode_jwt(jwt) {
        Ok(claims) => Some(claims.get_expiration()),
        Err(_) => None,
    }
}
