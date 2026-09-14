use super::decode_jwt::decode_jwt;

#[must_use]
pub fn get_timeout_from_jwt(jwt: &str) -> Option<usize> {
    decode_jwt(jwt).ok().map(|claims| claims.expiration())
}
