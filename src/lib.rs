use actix_web::{App, HttpServer, dev::Server, web};
use anyhow::{Context, Result};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tracing::info;
use tracing::subscriber::set_global_default;
use tracing_actix_web::TracingLogger;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry};
use uuid::Uuid;

use std::net::TcpListener;
use std::sync::LazyLock;
use std::time::Duration;

use crate::configuration::{Settings, get_configuration};
use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::routes::{health_check, subscribe};

pub mod configuration;
pub mod domain;
pub mod email_client;
pub mod routes;

// TODO: maybe move this to a more specfic tests file
pub static TEST_TRACING: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    let default_filter = "info";
    let filter = std::env::var("TEST_LOG").unwrap_or_else(|_| default_filter.to_string());

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_test_writer()
        .with_env_filter(filter)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");
});

pub struct AppHandle {
    pub handle: tokio::task::JoinHandle<Result<(), std::io::Error>>,
    pub pool: PgPool,
    pub config: Settings,
}

impl AppHandle {
    pub async fn run_until_stopped(self) -> Result<()> {
        self.handle.await??;
        Ok(())
    }
}

pub async fn spawn_prod_app() -> Result<AppHandle> {
    LogTracer::init().context("Failed to set logger")?;
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let formatting_layer = BunyanFormattingLayer::new("zero2prod".into(), std::io::stdout);
    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);

    set_global_default(subscriber).context("Failed to set subscriber")?;

    let config = get_configuration().context("Failed to read configuration")?;
    build_app(config).await
}

pub async fn spawn_test_app() -> Result<AppHandle> {
    // setup test logging
    LazyLock::force(&TEST_TRACING);
    let mut config = get_configuration().context("Failed to read configuration")?;
    apply_testing_overrides(&mut config);
    create_test_db(&config).await?;
    build_app(config).await
}

async fn build_app(mut config: Settings) -> Result<AppHandle> {
    let address = format!("{}:{}", config.app.host, config.app.port);
    let listener =
        TcpListener::bind(&address).context(format!("Failed to bind to address: {}", address))?;
    let port = listener.local_addr().unwrap().port();
    config.app.port = port;

    info!("Using config: {:?}", config);

    let conn = PgPoolOptions::new()
        .max_connections(config.database.max_connections.into())
        .connect_lazy_with(config.database.connection_options());

    let mail_client = EmailClient::new(
        SubscriberEmail::parse(config.email_client.sender_email.clone())
            .context("Invalid sender email address")?,
        config.email_client.base_url.clone(),
        config.email_client.auth_token.clone(),
        Duration::from_millis(config.email_client.timeout_milliseconds),
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
        config,
        pool: conn,
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

async fn create_test_db(config: &Settings) -> Result<()> {
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
