use super::decode_jwt::decode_jwt;

#[must_use]
pub fn get_user_id_from_jwt(jwt: &str) -> Option<String> {
    decode_jwt(jwt)
        .ok()
        .map(|claims| claims.user_id().to_string())
}
