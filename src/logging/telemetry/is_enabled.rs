/// Tells whether the environment asks for the trace export, given a variable lookup: an OTLP
/// endpoint is set and `OTEL_SDK_DISABLED` is not `true`.
#[must_use]
pub fn is_enabled(lookup: impl Fn(&str) -> Option<String>) -> bool {
    let set = |name: &str| lookup(name).is_some_and(|value| !value.trim().is_empty());
    let disabled =
        lookup("OTEL_SDK_DISABLED").is_some_and(|value| value.trim().eq_ignore_ascii_case("true"));
    !disabled && (set("OTEL_EXPORTER_OTLP_ENDPOINT") || set("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT"))
}

#[cfg(test)]
mod tests {
    use super::is_enabled;
    use std::collections::HashMap;

    fn lookup(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let vars: HashMap<String, String> = vars
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect();
        move |name| vars.get(name).cloned()
    }

    #[test]
    fn disabled_without_an_endpoint() {
        assert!(!is_enabled(lookup(&[])));
        assert!(!is_enabled(lookup(&[(
            "OTEL_EXPORTER_OTLP_ENDPOINT",
            "  "
        )])));
    }

    #[test]
    fn enabled_by_the_generic_or_the_traces_endpoint() {
        assert!(is_enabled(lookup(&[(
            "OTEL_EXPORTER_OTLP_ENDPOINT",
            "http://alloy:4318"
        )])));
        assert!(is_enabled(lookup(&[(
            "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
            "http://alloy:4318/v1/traces"
        )])));
    }

    #[test]
    fn sdk_disabled_wins_over_the_endpoint() {
        assert!(!is_enabled(lookup(&[
            ("OTEL_EXPORTER_OTLP_ENDPOINT", "http://alloy:4318"),
            ("OTEL_SDK_DISABLED", "TRUE"),
        ])));
        assert!(is_enabled(lookup(&[
            ("OTEL_EXPORTER_OTLP_ENDPOINT", "http://alloy:4318"),
            ("OTEL_SDK_DISABLED", "false"),
        ])));
    }
}
