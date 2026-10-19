//! `init_telemetry` installs the process-wide subscriber, so it is tested in its own binary.

use mairie360_api_lib::logging::telemetry::init_telemetry;

#[test]
fn init_exports_when_an_endpoint_is_set_and_installs_once() {
    temp_env::with_var(
        "OTEL_EXPORTER_OTLP_ENDPOINT",
        // Nothing listens on port 1: the spans are dropped at the export, the API keeps running.
        Some("http://127.0.0.1:1"),
        || {
            let guard = init_telemetry("telemetry-test");
            assert!(guard.is_some(), "an endpoint turns the export on");
            tracing::info_span!("request").in_scope(|| tracing::info!("exported nowhere"));

            assert!(
                init_telemetry("telemetry-test").is_none(),
                "a second init finds the subscriber installed and exports nothing"
            );
            // Flushes and shuts the provider down; the failed export is only logged.
            drop(guard);
        },
    );
}
