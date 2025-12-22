use anyhow::{Context, Result};
use sqlx::{ConnectOptions, PgPool};

use std::net::TcpListener;

use crate::{configuration::get_configuration, startup::run};

pub mod configuration;
pub mod routes;
pub mod startup;

pub struct AppHandle {
    pub handle: tokio::task::JoinHandle<Result<(), std::io::Error>>,
    pub address: String,
    pub conn: PgPool,
}

// TODO: change the return type to be a custom struct
pub async fn spawn_app() -> Result<AppHandle> {
    let config = get_configuration().context("Failed to read configuration")?;
    let address = format!("127.0.0.1:{}", config.application_port);

    let listener = TcpListener::bind(address).expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();

    let conn = {
        let connection_string = config.database.connection_string();
        PgPool::connect(&connection_string).await?
    };

    let server = run(listener, conn.clone()).expect("Failed to bind address");
    let handle = tokio::spawn(server);

    Ok(AppHandle {
        handle,
        conn,
        address: format!("http://127.0.0.1:{}", port),
    })
}
