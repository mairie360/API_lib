/**
 * Retrieves an environment variable by name.
 * Returns `Some(value)` if the variable is set, or `None` if it is not.
 */
pub fn get_env_var(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/**
 * Retrieves an environment variable by name, or exits the program with an error message if it is not set.
 * This is useful for critical environment variables that must be present for the application to run.
 */
pub fn get_critical_env_var(name: &str) -> String {
    match std::env::var(name) {
        Ok(val) => val,
        Err(_) => {
            panic!("Critical environment variable '{}' is not set", name);
        }
    }
}
