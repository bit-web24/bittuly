use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{Resource, trace as sdk_trace};
use std::sync::OnceLock;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

static PROVIDER: OnceLock<sdk_trace::SdkTracerProvider> = OnceLock::new();

pub fn init_tracing(service_name: &'static str) {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());

    let resource = Resource::builder().with_service_name(service_name).build();

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .expect("Failed to build OTLP span exporter");

    let provider = sdk_trace::SdkTracerProvider::builder()
        .with_resource(resource)
        .with_batch_exporter(exporter)
        .build();

    let tracer = provider.tracer(service_name);

    let _ = PROVIDER.set(provider.clone());
    opentelemetry::global::set_tracer_provider(provider);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{service_name}=debug,shared=debug,tower_http=debug").into()
            }),
        )
        .with(tracing_subscriber::fmt::layer()) // console output (unchanged)
        .with(tracing_opentelemetry::layer().with_tracer(tracer)) // → Jaeger
        .init();
}

pub fn shutdown_tracing() {
    if let Some(provider) = PROVIDER.get() {
        let _ = provider.shutdown();
    }
}

pub fn inject_context(carrier: &mut std::collections::HashMap<String, String>) {
    use opentelemetry::propagation::TextMapPropagator;
    let propagator = opentelemetry_sdk::propagation::TraceContextPropagator::new();
    propagator.inject_context(&opentelemetry::Context::current(), carrier);
}

pub fn extract_context(
    carrier: &std::collections::HashMap<String, String>,
) -> opentelemetry::Context {
    use opentelemetry::propagation::TextMapPropagator;
    let propagator = opentelemetry_sdk::propagation::TraceContextPropagator::new();
    propagator.extract(carrier)
}
