pub fn get_critical_env_var(name: &str) -> String {
    match std::env::var(name) {
        Ok(val) => val,
        Err(_) => {
            panic!("Critical environment variable '{}' is not set", name);
        }
    }
}
