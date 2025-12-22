use std::time::{SystemTime, UNIX_EPOCH};

/**
 * Checks if the JWT token is still valid based on its expiration time.
 *
 * # Arguments
 * * `jwt_expiration` - The expiration time of the JWT token as a Unix timestamp in seconds.
 *
 * # Returns
 * * `Ok(true)` if the token is still valid (not expired).
 * * `Ok(false)` if the token has expired.
 * * `Err(String)` if there was an error retrieving the current system time.
 */
pub fn verify_jwt_timeout(jwt_expiration: usize) -> Result<bool, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("SystemTime error: {:?}", e))?
        .as_secs() as usize;

    if jwt_expiration < now {
        Ok(false)
    } else {
        Ok(true)
    }
}
