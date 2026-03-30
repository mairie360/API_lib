use crate::database::db_interface::DatabaseQueryView;
use std::fmt::Display;
use std::net::IpAddr;

/**
 * Query view to check if a user exists by their ID.
 */
pub struct IsSessionTokenValidQueryView {
    user_id: u64,
    session_token: String,
    ip_address: IpAddr,
}

impl IsSessionTokenValidQueryView {
    pub fn new(user_id: u64, session_token: String, ip_address: IpAddr) -> Self {
        Self {
            user_id,
            session_token,
            ip_address,
        }
    }
    pub fn get_user_id(&self) -> u64 {
        self.user_id
    }
    pub fn get_session_token(&self) -> &str {
        &self.session_token
    }
    pub fn get_ip_address(&self) -> &IpAddr {
        &self.ip_address
    }
}

impl DatabaseQueryView for IsSessionTokenValidQueryView {
    fn get_request(&self) -> String {
        "SELECT EXISTS(
            SELECT 1 FROM v_sessions
            WHERE user_id = $1
                AND token_hash = $2
                AND ip_address = $3::inet
                AND is_active = true
                AND user_is_archived = false
            ) AS is_valid"
            .to_string()
    }
}

impl Display for IsSessionTokenValidQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IsSessionTokenValidQueryView: user_id = {}, session_token = {}, ip_address = {}",
            self.user_id, self.session_token, self.ip_address
        )
    }
}
