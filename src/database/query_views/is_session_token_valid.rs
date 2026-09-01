use crate::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;
use std::net::IpAddr;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IsSessionTokenValidQueryView {
    params: Vec<QueryParam>,
}

impl IsSessionTokenValidQueryView {
    pub fn new(user_id: u64, session_token: String, ip_address: IpAddr) -> Self {
        Self {
            params: vec![
                QueryParam::I32(user_id as i32),
                QueryParam::Text(session_token),
                QueryParam::IpAddr(ip_address),
            ],
        }
    }
    pub fn get_user_id(&self) -> u64 {
        self.params[0].as_i32() as u64
    }
    pub fn get_session_token(&self) -> &str {
        self.params[1].as_text()
    }
    pub fn get_ip_address(&self) -> IpAddr {
        self.params[2].as_ipaddr()
    }
}

impl ApiRequestDto for IsSessionTokenValidQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT EXISTS(
            SELECT 1 FROM v_sessions
            WHERE user_id = $1
                AND token_hash = $2
                AND ip_address = $3::inet
                AND is_active = true
            ) AS is_valid"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for IsSessionTokenValidQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IsSessionTokenValidQueryView: user_id = {}, session_token = {}, ip_address = {}",
            self.get_user_id(),
            self.get_session_token(),
            self.get_ip_address()
        )
    }
}
