use actix_web::cookie::Key;
use actix_web::{App, HttpServer, dev::Server, web};
use actix_web_flash_messages::FlashMessagesFramework;
use actix_web_flash_messages::storage::CookieMessageStore;
use anyhow::{Context, Result};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tracing::info;
use tracing::subscriber::set_global_default;
use tracing_actix_web::TracingLogger;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry};

use std::net::TcpListener;
use std::time::Duration;

use crate::configuration::{Settings, get_configuration};
use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::routes::{
    confirm_subscription, health_check, home::home, publish_newsletter, subscribe,
};
use crate::routes::{login_form, login_post};

#[derive(Clone, Debug, Deserialize)]
pub struct HmacSecret(pub Secret<String>);

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
    // don't override email client in production
    build_app(config).await
}

pub async fn build_app(mut config: Settings) -> Result<AppHandle> {
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

    let server = run(listener, conn.clone(), mail_client, config.clone())
        .context("Failed to start server")?;

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

fn run(
    listener: TcpListener,
    connection: PgPool,
    email_client: EmailClient,
    config: Settings,
) -> Result<Server> {
    let hmac_secret = config.app.hmac_secret.clone();
    let message_store =
        CookieMessageStore::builder(Key::from(hmac_secret.0.expose_secret().as_bytes())).build();
    let message_framework = FlashMessagesFramework::builder(message_store).build();

    let connection = web::Data::new(connection);
    let email_client = web::Data::new(email_client);
    let app_data = web::Data::new(config);

    Ok(HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .wrap(message_framework.clone())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .route(
                "/subscriptions/confirm",
                web::get().to(confirm_subscription),
            )
            .route("/newsletters", web::post().to(publish_newsletter))
            .route("/", web::get().to(home))
            .route("/login", web::get().to(login_form))
            .route("/login", web::post().to(login_post))
            .app_data(connection.clone())
            .app_data(email_client.clone())
            .app_data(app_data.clone())
    })
    .listen(listener)?
    .run())
}
