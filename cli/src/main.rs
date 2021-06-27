use std::env::var;

use color_eyre::Result;
use opentelemetry::sdk;
use opentelemetry_semantic_conventions as semcov;
use tracing_error::ErrorLayer;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

fn setup_tracing() -> Result<()> {
    color_eyre::install()?;
    let tracer = opentelemetry_otlp::new_pipeline()
        .with_env()
        .with_trace_config(sdk::trace::config().with_resource(sdk::Resource::new(vec![
            semcov::resource::SERVICE_NAME.string("teleport-service-controller"),
        ])))
        .with_tonic()
        .install_simple()?;

    let error = ErrorLayer::default();
    let env_filter = EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("info"))?;
    let stdout = tracing_subscriber::fmt::layer();
    let telemetry = match var("OTEL_EXPORTER_OTLP_ENDPOINT") {
        Ok(endpoint) => {
            eprintln!("Sending traces to {}", endpoint);
            Some(tracing_opentelemetry::layer().with_tracer(tracer))
        }
        _ => None,
    };

    tracing_subscriber::registry()
        .with(error)
        .with(env_filter)
        .with(stdout)
        .with(telemetry)
        .init();

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_tracing()?;
    let controller = lib::Controller::new().await?;
    controller.run().await?;
    Ok(())
}
