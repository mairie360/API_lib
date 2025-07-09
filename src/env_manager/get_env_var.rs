/**
 * Retrieves an environment variable by name.
 * Returns `Some(value)` if the variable is set, or `None` if it is not.
 */
pub fn get_env_var(name: &str) -> Option<String> {
    std::env::var(name).ok()
}
