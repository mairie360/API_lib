use super::get_env_var;

/// Environment variable that serves Swagger UI (`/swagger-ui/`) and the `OpenAPI` document
/// (`/api-docs/openapi.json`) when set to `true` (MAIR-424).
///
/// Dev and test stacks set it (ZAP,
/// k6 and newman read the spec from the running API); production leaves it unset: consumers get
/// the contract from the published `@mairie360/<name>-api-openapi` package.
pub const API_DOCS_ENABLED: &str = "API_DOCS_ENABLED";

/// Whether [`API_DOCS_ENABLED`] enables the docs: only `true` (case-insensitive, surrounding
/// spaces ignored) does; unset or any other value keeps them off.
///
/// ```ignore
/// let docs_enabled = api_docs_enabled();
/// App::new().configure(|cfg| {
///     if docs_enabled {
///         cfg.service(SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", ApiDoc::openapi()));
///     }
/// })
/// ```
#[must_use]
pub fn api_docs_enabled() -> bool {
    is_api_docs_value_enabled(get_env_var(API_DOCS_ENABLED).as_deref())
}

/// [`api_docs_enabled`] on an explicit value of [`API_DOCS_ENABLED`].
#[must_use]
pub fn is_api_docs_value_enabled(value: Option<&str>) -> bool {
    value.is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
}

#[cfg(test)]
mod tests {
    use super::is_api_docs_value_enabled;

    #[test]
    fn only_true_enables_the_docs() {
        for value in ["true", "TRUE", " True "] {
            assert!(is_api_docs_value_enabled(Some(value)), "{value:?}");
        }
        for value in [
            None,
            Some(""),
            Some("1"),
            Some("yes"),
            Some("false"),
            Some("truee"),
        ] {
            assert!(!is_api_docs_value_enabled(value), "{value:?}");
        }
    }
}
