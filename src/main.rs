use anyhow::Result;
use tracing::info;
use zero2prod::spawn_prod_app;

#[tokio::main]
async fn main() -> Result<()> {
    let app = spawn_prod_app().await?;
    info!("Server running on {}", app.config.app_address());
    app.run_until_stopped().await?;
    Ok(())
}
