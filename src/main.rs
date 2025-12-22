use anyhow::{Result, bail};

use zero2prod::spawn_app;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter(
        tracing_subscriber::EnvFilter::from_default_env()
            .add_directive(tracing::Level::INFO.into()),
    ).init();

    if let Ok(app) = spawn_app().await {
        println!("Server running on {}", app.address);
        Ok(app.handle.await??)
    } else {
        bail!("Failed to spawn application")
    }
}
