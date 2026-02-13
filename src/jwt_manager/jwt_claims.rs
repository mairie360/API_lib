use serde::{Deserialize, Serialize};

/**
 * This struct represents the claims of a JSON Web Token (JWT).
 * It contains the subject (user ID) and expiration time.
 * The `sub` field is the subject of the token, typically the user ID.
 * The `exp` field is the expiration time of the token in seconds since the epoch.
 */
#[derive(Serialize, Deserialize)]
pub struct Claims {
    sub: String,
    exp: usize,
}

impl Claims {
    /**
     * Creates a new instance of `Claims`.
     */
    pub fn new(user_id: String, expiration: usize) -> Self {
        Claims {
            sub: user_id,
            exp: expiration,
        }
    }

    /**
     * Returns the user ID from the claims.
     * This is typically the subject of the JWT.
     * # Returns
     * * `&str` - A reference to the user ID (subject) contained in the claims.
     */
    pub fn get_user_id(&self) -> &str {
        &self.sub
    }

    /**
     * Returns the expiration time from the claims.
     * This is the time in seconds since the epoch when the token expires.
     * # Returns
     * * `usize` - The expiration time in seconds.
     */
    pub fn get_expiration(&self) -> usize {
        self.exp
    }
}

impl std::fmt::Display for Claims {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Claims {{ sub: {}, exp: {} }}", self.sub, self.exp)
    }
}
