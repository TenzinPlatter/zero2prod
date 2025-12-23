use actix_web::{App, HttpServer, dev::Server, web};
use anyhow::{Context, Result};
use secrecy::ExposeSecret;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tracing_actix_web::TracingLogger;
use uuid::Uuid;

use std::net::TcpListener;

use crate::configuration::{Settings, get_configuration};
use crate::routes::{health_check, subscribe};

pub mod configuration;
pub mod routes;

pub struct AppHandle {
    pub handle: tokio::task::JoinHandle<Result<(), std::io::Error>>,
    pub address: String,
    pub pool: PgPool,
}

pub async fn spawn_prod_app() -> Result<AppHandle> {
    spawn_app(true).await
}

pub async fn spawn_test_app() -> Result<AppHandle> {
    spawn_app(false).await
}

pub(crate) async fn spawn_app(prod: bool) -> Result<AppHandle> {
    let mut config = get_configuration().context("Failed to read configuration")?;
    let address = format!("{}:{}", config.app.host, config.app.port);

    let listener =
        TcpListener::bind(&address).context(format!("Failed to bind to address: {}", address))?;
    let port = listener.local_addr().unwrap().port();

    if !prod {
        apply_testing_overrides(&mut config);
        setup_testing_db(&config).await?;
    }

    let conn = PgPoolOptions::new()
        .max_connections(config.database.max_connections.into())
        .connect_lazy(config.database.connection_string().expose_secret())
        .context("Failed to create DB connection pool")?;

    let server = run(listener, conn.clone()).context("Failed to start server")?;
    let handle = tokio::spawn(server);

    // Migrate the database
    sqlx::migrate!("./migrations")
        .run(&conn)
        .await
        .expect("Failed to migrate the database");

    Ok(AppHandle {
        handle,
        pool: conn,
        address: format!("http://127.0.0.1:{}", port),
    })
}

fn run(listener: TcpListener, connection: PgPool) -> Result<Server> {
    let connection = web::Data::new(connection);
    Ok(HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(connection.clone())
    })
    .listen(listener)?
    .run())
}

fn apply_testing_overrides(config: &mut Settings) {
    config.database.database_name = Uuid::new_v4().to_string();
    config.app.port = 0;
}

async fn setup_testing_db(config: &Settings) -> Result<()> {
    // For tests: create a unique database per test
    let postgres_connection = config.database.postgres_connection_string();
    let db_pool = PgPool::connect_lazy(postgres_connection.expose_secret())?;

    // Create the database
    // SAFETY: no injections as we just generated the DB name using Uuid, also we are required to
    // use format! for DDL, parameterized query doesn't work
    sqlx::query(&format!(
        r#"CREATE DATABASE "{}";"#,
        config.database.database_name
    ))
    .execute(&db_pool)
    .await?;

    db_pool.close().await;
    Ok(())
}
