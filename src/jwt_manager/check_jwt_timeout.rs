use std::time::{SystemTime, UNIX_EPOCH};

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
