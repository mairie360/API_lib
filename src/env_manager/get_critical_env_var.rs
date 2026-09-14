/// Lit une variable d'environnement indispensable au démarrage du service.
///
/// # Panics
///
/// Panique si la variable `name` n'est pas définie ou n'est pas de l'UTF-8 valide.
#[must_use]
pub fn get_critical_env_var(name: &str) -> String {
    std::env::var(name)
        .unwrap_or_else(|_| panic!("Critical environment variable '{name}' is not set"))
}
