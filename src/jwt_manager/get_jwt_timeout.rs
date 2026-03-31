use crate::env_manager::get_env_var;

pub fn get_jwt_timeout() -> Result<usize, String> {
    match get_env_var("JWT_TIMEOUT") {
        Some(secret) => secret
            .parse::<usize>()
            .map_err(|_| "JWT_TIMEOUT is not a valid usize".to_string()),
        None => Err("JWT_TIMEOUT environment variable not set".to_string()),
    }
}
