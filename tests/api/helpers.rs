use std::sync::LazyLock;

use anyhow::{Context, Result};
use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use fake::{Fake, faker};
use reqwest::{Response, Url};
use serde::Serialize;
use sqlx::PgPool;
use tracing::debug;
use uuid::Uuid;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

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
    pub user: TestUser,
    pub api_client: reqwest::Client,
}

pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
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
    let user = add_test_user(&app.pool).await?;
    let api_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()?;

    Ok(TestApp {
        handle: app.handle,
        pool: app.pool,
        config: app.config,
        email_server,
        user,
        api_client,
    })
}

impl TestApp {
    pub(crate) async fn post_subscriptions(&self, body: String) -> Result<reqwest::Response> {
        Ok(self
            .api_client
            .post(format!("{}/subscriptions", self.config.app_address()))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await?)
    }

    pub(crate) async fn post_newsletters<T: ToString>(
        &self,
        body: serde_json::Value,
        creds: Option<(T, T)>,
    ) -> Result<reqwest::Response> {
        let request = self
            .api_client
            .post(format!("{}/newsletters", self.config.app_address()))
            .header("Content-Type", "application/json")
            .json(&body);

        match creds {
            Some((username, password)) => Ok(request
                .basic_auth(username.to_string(), Some(password.to_string()))
                .send()
                .await?),
            None => Ok(request.send().await?),
        }
    }

    pub async fn create_confirmed_subscriber(&self) -> Result<()> {
        let confirmation_links = create_unconfirmed_subscriber(self).await?;
        self.api_client
            .get(confirmation_links.plain_text.as_str())
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn post_login<Body>(&self, body: &Body) -> Result<reqwest::Response>
    where
        Body: serde::Serialize,
    {
        Ok(self
            .api_client
            .post(format!("{}/login", self.config.app_address()))
            .form(body)
            .send()
            .await?)
    }

    pub async fn get_test_user_creds(&self) -> (String, String) {
        (self.user.username.clone(), self.user.password.clone())
    }

    pub async fn get_login_html(&self) -> Result<String> {
        Ok(self
            .api_client
            .get(format!("{}/login", self.config.app_address()))
            .send()
            .await?
            .text()
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

    let text_link = get_link(body.get("text").unwrap().as_str().unwrap());
    let html_link = get_link(body.get("html").unwrap().as_str().unwrap());

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

#[derive(Serialize)]
struct SubscriptionFormData {
    name: String,
    email: String,
}

pub fn get_fake_subscription_body() -> String {
    let form = SubscriptionFormData {
        name: faker::name::en::Name().fake(),
        email: faker::internet::en::SafeEmail().fake(),
    };

    serde_urlencoded::to_string(&form).unwrap()
}

pub async fn create_unconfirmed_subscriber(
    app: &crate::helpers::TestApp,
) -> Result<ConfirmationLinks> {
    let body = get_fake_subscription_body();
    let _mock_guard = Mock::given(path("/emails"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber email")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscriptions(body).await?.error_for_status()?;

    let request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();

    get_email_links(request)
}

pub async fn add_test_user(pool: &PgPool) -> Result<TestUser> {
    let password = Uuid::new_v4().to_string();
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(15000, 2, 1, None).unwrap(),
    )
    .hash_password(password.as_bytes(), &salt)?
    .to_string();

    let user = TestUser {
        user_id: Uuid::new_v4(),
        username: Uuid::new_v4().to_string(),
        password,
    };

    sqlx::query!(
        r#"
        INSERT INTO users (user_id, username, password_hash)
        VALUES ($1, $2, $3)
        "#,
        user.user_id,
        user.username,
        hash,
    )
    .execute(pool)
    .await?;

    Ok(user)
}

pub fn assert_is_redirect_to(response: &Response, to: &str) {
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), to);
}
