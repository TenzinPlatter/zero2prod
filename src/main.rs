use anyhow::Result;

use zero2prod::spawn_app;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let app = spawn_app().await?;
    println!("Server running on {}", app.address);
    app.handle.await??;
    Ok(())
}
