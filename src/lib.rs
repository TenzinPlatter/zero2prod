use anyhow::{Context, Result};
use sqlx::PgPool;
use uuid::Uuid;

use std::net::TcpListener;

use crate::{
    configuration::{Settings, get_configuration},
    startup::run,
};

pub mod configuration;
pub mod routes;
pub mod startup;

pub struct AppHandle {
    pub handle: tokio::task::JoinHandle<Result<(), std::io::Error>>,
    pub address: String,
    pub conn: PgPool,
}

pub async fn spawn_prod_app() -> Result<AppHandle> {
    spawn_app(true).await
}

pub async fn spawn_test_app() -> Result<AppHandle> {
    spawn_app(false).await
}

pub(crate) async fn spawn_app(prod: bool) -> Result<AppHandle> {
    let overrides = match prod {
        true => |settings| settings,
        false => |mut settings: Settings| {
            settings.application_port = 0;
            settings.database.database_name = Uuid::new_v4().to_string();
            settings
        },
    };

    let config = get_configuration(overrides).context("Failed to read configuration")?;
    let address = format!("127.0.0.1:{}", config.application_port);

    let listener =
        TcpListener::bind(&address).context(format!("Failed to bind to address: {}", address))?;
    let port = listener.local_addr().unwrap().port();

    let conn = if !prod {
        // For tests: create a unique database per test
        let postgres_connection = config.database.postgres_connection_string();
        let db_pool = PgPool::connect(&postgres_connection).await?;

        // Create the database
        sqlx::query(&format!(
            r#"CREATE DATABASE "{}";"#,
            config.database.database_name
        ))
        .execute(&db_pool)
        .await?;

        // Now connect to the newly created database
        PgPool::connect(&config.database.connection_string()).await?
    } else {
        // For prod: just connect to the configured database
        PgPool::connect(&config.database.connection_string()).await?
    };

    let server = run(listener, conn.clone()).context("Failed to bind address")?;
    let handle = tokio::spawn(server);

    // Migrate the database
    sqlx::migrate!("./migrations")
        .run(&conn)
        .await
        .expect("Failed to migrate the database");

    Ok(AppHandle {
        handle,
        conn,
        address: format!("http://127.0.0.1:{}", port),
    })
}
