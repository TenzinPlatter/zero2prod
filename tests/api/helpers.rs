use std::sync::LazyLock;

use anyhow::{Context, Result};

use reqwest::Url;
use sqlx::PgPool;
use tracing::debug;
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::{
    configuration::{Settings, get_configuration},
    startup::build_app,
};

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

pub struct TestApp {
    #[allow(dead_code)]
    pub handle: tokio::task::JoinHandle<Result<(), std::io::Error>>,
    pub pool: PgPool,
    pub config: Settings,
    pub email_server: MockServer,
}

pub struct ConfirmationLinks {
    pub html: Url,
    pub plain_text: Url,
}

pub async fn spawn_test_app() -> Result<TestApp> {
    // setup test logging
    LazyLock::force(&TEST_TRACING);
    let mut config = get_configuration().context("Failed to read configuration")?;
    debug!("Original config: {:?}", config);

    let email_server = MockServer::start().await;
    apply_testing_overrides(&mut config, &email_server);
    debug!("Testing config: {:?}", config);

    create_test_db(&config).await?;
    debug!("Created test db");

    let app = build_app(config).await?;
    Ok(TestApp {
        handle: app.handle,
        pool: app.pool,
        config: app.config,
        email_server,
    })
}

impl TestApp {
    pub(crate) async fn post_subscriptions(&self, body: String) -> Result<reqwest::Response> {
        Ok(reqwest::Client::new()
            .post(format!("{}/subscriptions", self.config.app_address()))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await?)
    }
}

pub fn get_email_links(request: &wiremock::Request) -> Result<ConfirmationLinks> {
    let get_link = |s: &str| {
        let links: Vec<_> = linkify::LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect();
        assert_eq!(links.len(), 1);
        let raw = links[0].as_str().to_owned();
        let confirmation = Url::parse(&raw).unwrap();
        assert_eq!(confirmation.host_str().unwrap(), "127.0.0.1");
        confirmation
    };

    let body: serde_json::Value = serde_json::from_slice(&request.body)?;

    let text_link = get_link(body.get("TextBody").unwrap().as_str().unwrap());
    let html_link = get_link(body.get("HtmlBody").unwrap().as_str().unwrap());

    Ok(ConfirmationLinks {
        html: html_link,
        plain_text: text_link,
    })
}

fn apply_testing_overrides(config: &mut Settings, email_server: &MockServer) {
    config.database.database_name = Uuid::new_v4().to_string();
    config.app.port = 0;
    config.email_client.base_url = email_server.uri();
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
