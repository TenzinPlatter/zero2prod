use anyhow::{Context, Result};
use tracing::{info, subscriber::set_global_default};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt};
use zero2prod::spawn_prod_app;

#[tokio::main]
async fn main() -> Result<()> {
    LogTracer::init().context("Failed to set logger")?;

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let formatting_layer = BunyanFormattingLayer::new("zero2prod".into(), std::io::stdout);

    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);

    set_global_default(subscriber).context("Failed to set subscriber")?;

    let app = spawn_prod_app().await?;
    info!("Server running on {}", app.address);
    app.handle.await??;
    Ok(())
}
