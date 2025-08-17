#![allow(unused_imports)] // some used only for telemetry feature
use opentelemetry::trace::{TraceId, TracerProvider};
use opentelemetry_sdk::{Resource, trace as sdktrace};
use sdktrace::{SdkTracer, SdkTracerProvider};
use snafu::{ResultExt, Whatever};
use tracing_subscriber::{EnvFilter, Registry, prelude::*};
use validator::Validate;
use validator_derive::Validate;

#[derive(Debug, Validate)]
#[derive(Default)]
pub struct TelemetryConfig {
    /// Enabled telemetry
    enabled: bool,
    /// The endpoint to send telemetry to
    #[validate(url)]
    endpoint: Option<String>,
}


impl TelemetryConfig {
    pub fn from_env() -> Result<Self, Whatever> {
        let config = match std::env::var("OPENTELEMETRY_ENDPOINT_URL") {
            Ok(endpoint) => Self {
                enabled: true,
                endpoint: Some(endpoint),
            },
            Err(_) => Self {
                enabled: false,
                endpoint: None,
            },
        };
        config.validate().whatever_context("Invalid config")?;
        Ok(config)
    }

    pub fn new(endpoint: Option<String>) -> Result<Self, Whatever> {
        let config = Self {
            enabled: endpoint.is_some(),
            endpoint,
        };
        config.validate().whatever_context("Invalid config")?;
        Ok(config)
    }
}

///  Fetch an opentelemetry::trace::TraceId as hex through the full tracing stack
pub fn get_trace_id() -> TraceId {
    use opentelemetry::trace::TraceContextExt as _; // opentelemetry::Context -> opentelemetry::trace::Span
    use tracing_opentelemetry::OpenTelemetrySpanExt as _; // tracing::Span to opentelemetry::Context
    tracing::Span::current()
        .context()
        .span()
        .span_context()
        .trace_id()
}

fn resource() -> Resource {
    use opentelemetry::KeyValue;
    Resource::builder()
        .with_service_name(env!("CARGO_PKG_NAME"))
        .with_attribute(KeyValue::new("service.version", env!("CARGO_PKG_VERSION")))
        .build()
}

fn init_tracer(endpoint: &str) -> SdkTracer {
    use opentelemetry_otlp::{SpanExporter, WithExportConfig};
    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .unwrap();

    let provider = SdkTracerProvider::builder()
        .with_resource(resource())
        .with_batch_exporter(exporter)
        .build();

    provider.tracer("tracing-otel-subscriber")
}

/// Initialize tracing
pub async fn init(config: &TelemetryConfig) {
    let logger = tracing_subscriber::fmt::layer().compact();
    let env_filter = EnvFilter::try_from_default_env()
        .or(EnvFilter::try_new("info"))
        .unwrap();

    // Decide on layers
    let reg = Registry::default().with(env_filter).with(logger);

    if config.enabled && config.endpoint.is_some() {
        let otel = tracing_opentelemetry::OpenTelemetryLayer::new(init_tracer(
            config.endpoint.as_ref().unwrap(),
        ));
        reg.with(otel).init();
    } else {
        reg.init();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test_log::test(tokio::test)]
    async fn telemetry_config_from_env_success() {
        use super::*;
        temp_env::with_var(
            "OPENTELEMETRY_ENDPOINT_URL",
            Some("http://localhost:4317"),
            || {
                let config = TelemetryConfig::from_env().unwrap();
                assert_eq!(
                    config.endpoint,
                    Some("http://localhost:4317".to_string()),
                    "endpoint"
                );
                assert!(config.enabled, "enabled when endpoint");
            },
        );

        temp_env::with_var("OPENTELEMETRY_ENDPOINT_URL", None::<&str>, || {
            let config = TelemetryConfig::from_env().unwrap();
            assert_eq!(config.endpoint, None, "no endpoint");
            assert!(!config.enabled, "disabled when no endpoint");
        });
    }

    #[test_log::test(tokio::test)]
    async fn telemetry_config_from_env_failure() {
        use super::*;
        temp_env::with_var("OPENTELEMETRY_ENDPOINT_URL", Some("invalid"), || {
            let result = TelemetryConfig::from_env();
            assert!(result.is_err(), "invalid endpoint");
        });
    }
}
