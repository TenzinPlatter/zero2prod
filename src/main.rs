use std::net::TcpListener;

use anyhow::{Context, Result};

use zero2prod::{configuration::get_configuration, startup::run};

#[tokio::main]
async fn main() -> Result<()> {
    let config = get_configuration().context("Failed to read configuration")?;

    let address = format!("127.0.0.1:{}", config.application_port);
    let listener = TcpListener::bind(address).context("Failed to bind to address")?;
    println!("Server running on http://127.0.0.1:{}", config.application_port);
    Ok(run(listener)?.await?)
}
