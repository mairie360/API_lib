use super::decode_jwt::decode_jwt;

pub fn get_user_id_from_jwt(jwt: &str) -> Option<String> {
    match decode_jwt(jwt) {
        Ok(claims) => Some(claims.get_user_id().to_string()),
        Err(_) => None,
    }
}
