use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Claims {
    sub: String,
    exp: usize,
}

impl Claims {
    pub fn new(user_id: String, expiration: usize) -> Self {
        Claims {
            sub: user_id,
            exp: expiration,
        }
    }

    pub fn get_user_id(&self) -> &str {
        &self.sub
    }

    pub fn get_expiration(&self) -> usize {
        self.exp
    }
}

impl std::fmt::Display for Claims {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Claims {{ sub: {}, exp: {} }}", self.sub, self.exp)
    }
}
