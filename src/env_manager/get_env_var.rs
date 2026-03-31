pub fn get_env_var(name: &str) -> Option<String> {
    std::env::var(name).ok()
}
