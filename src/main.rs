use anyhow::{Result, bail};

use zero2prod::spawn_app;

#[tokio::main]
async fn main() -> Result<()> {
    if let Ok((handle, address)) = spawn_app() {
        println!("Server running on {}", address);
        Ok(handle.await??)
    } else {
        bail!("Failed to spawn application")
    }
}
