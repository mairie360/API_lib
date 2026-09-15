use crate::database::db_interface::{id_from_sql, id_to_sql, ApiRequestDto, QueryParam};
use std::fmt::Display;
use std::net::IpAddr;

/// Vérifie qu'un refresh token correspond à une session active de l'utilisateur.
///
/// L'IP n'est **pas** un critère de validité : les APIs ne voient que l'IP du BFF, et un client
/// dont l'IP change (mobile passant du Wi-Fi à la 4G, VPN) doit garder sa session. Le token
/// (aléatoire et unique) suffit à identifier la session. L'IP reste acceptée par [`Self::new`]
/// et exposée par [`Self::get_ip_address`] pour ne pas casser les appelants existants.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IsSessionTokenValidQueryView {
    ip_address: IpAddr,
    params: Vec<QueryParam>,
}

impl IsSessionTokenValidQueryView {
    #[must_use]
    pub fn new(user_id: u64, session_token: String, ip_address: IpAddr) -> Self {
        Self {
            ip_address,
            params: vec![
                QueryParam::I32(id_to_sql(user_id)),
                QueryParam::Text(session_token),
            ],
        }
    }
    #[must_use]
    pub fn get_user_id(&self) -> u64 {
        id_from_sql(self.params[0].as_i32())
    }
    #[must_use]
    pub fn get_session_token(&self) -> &str {
        self.params[1].as_text()
    }
    #[must_use]
    pub const fn get_ip_address(&self) -> IpAddr {
        self.ip_address
    }
}

impl ApiRequestDto for IsSessionTokenValidQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT EXISTS(
            SELECT 1 FROM v_sessions
            WHERE user_id = $1
                AND token_hash = $2
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
