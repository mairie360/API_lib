use super::decode_jwt::decode_jwt;

#[must_use]
pub fn get_role_from_jwt(jwt: &str) -> Option<String> {
    decode_jwt(jwt).ok().map(|claims| claims.role().to_string())
}
