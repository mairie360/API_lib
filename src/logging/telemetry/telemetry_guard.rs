use opentelemetry_sdk::trace::SdkTracerProvider;

/// Keeps the tracer provider alive: dropping it flushes the spans still buffered. Hold it in
/// `main` until the server stops (`let _telemetry = init_telemetry(…);`).
#[derive(Debug)]
pub struct TelemetryGuard {
    provider: SdkTracerProvider,
}

impl TelemetryGuard {
    pub(super) const fn new(provider: SdkTracerProvider) -> Self {
        Self { provider }
    }
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Err(error) = self.provider.shutdown() {
            tracing::warn!("OpenTelemetry shutdown failed: {error}");
        }
    }
}
