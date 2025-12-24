use actix_web::{App, HttpServer, dev::Server, web};
use anyhow::{Context, Result};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tracing::info;
use tracing_actix_web::TracingLogger;
use uuid::Uuid;

use std::net::TcpListener;

use crate::configuration::{Settings, get_configuration};
use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::routes::{health_check, subscribe};

pub mod configuration;
pub mod domain;
pub mod email_client;
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

    info!("Using config: {:?}", config);

    let conn = PgPoolOptions::new()
        .max_connections(config.database.max_connections.into())
        .connect_lazy_with(config.database.connection_options());

    let mail_client = EmailClient::new(
        SubscriberEmail::parse(config.email_client.sender_email.clone())
            .context("Invalid sender email address")?,
        config.email_client.base_url.clone(),
        config.email_client.auth_token.clone(),
    );

    let server = run(listener, conn.clone(), mail_client).context("Failed to start server")?;
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

fn run(listener: TcpListener, connection: PgPool, email_client: EmailClient) -> Result<Server> {
    let connection = web::Data::new(connection);
    let email_client = web::Data::new(email_client);
    Ok(HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(connection.clone())
            .app_data(email_client.clone())
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
    let postgres_connection = config.database.postgres_connection_options();
    let db_pool = PgPool::connect_lazy_with(postgres_connection);

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
