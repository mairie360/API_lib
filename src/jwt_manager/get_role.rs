use super::decode_jwt::decode_jwt;

pub fn get_role_from_jwt(jwt: &str) -> Option<String> {
    match decode_jwt(jwt) {
        Ok(claims) => Some(claims.role().to_string()),
        Err(_) => None,
    }
}
