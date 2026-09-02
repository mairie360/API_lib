use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Claims {
    sub: String,
    pub role: String,
    exp: usize,
}

impl Claims {
    pub fn new(user_id: &str, role: &str, expiration: usize) -> Self {
        Claims {
            sub: user_id.to_string(),
            role: role.to_string(),
            exp: expiration,
        }
    }

    pub fn user_id(&self) -> &str {
        &self.sub
    }

    pub fn expiration(&self) -> usize {
        self.exp
    }

    pub fn role(&self) -> &str {
        &self.role
    }
}

impl std::fmt::Display for Claims {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Claims {{ sub: {}, role: {}, exp: {} }}",
            self.sub, self.role, self.exp
        )
    }
}
