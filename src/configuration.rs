use anyhow::{Result, bail};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::PgConnectOptions;

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub app: ApplicationSettings,
    pub email_client: EmailClientSettings,
}

#[derive(Deserialize, Debug)]
pub struct ApplicationSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
}

#[derive(Deserialize, Debug)]
pub struct EmailClientSettings {
    pub base_url: String,
    pub sender_email: String,
    pub auth_token: Secret<String>,
}

#[derive(Deserialize, Debug)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub database_name: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub max_connections: u8,
}

impl DatabaseSettings {
    pub fn postgres_connection_options(&self) -> PgConnectOptions {
        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(self.password.expose_secret())
            .port(self.port)
            .database("postgres")
            .ssl_mode(
                if std::env::var("APP_ENVIRONMENT").unwrap_or_default() == "PRODUCTION" {
                    sqlx::postgres::PgSslMode::Require
                } else {
                    sqlx::postgres::PgSslMode::Prefer
                },
            )
    }

    pub fn connection_options(&self) -> PgConnectOptions {
        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(self.password.expose_secret())
            .port(self.port)
            .database(&self.database_name)
            .ssl_mode(
                if std::env::var("APP_ENVIRONMENT").unwrap_or_default() == "PRODUCTION" {
                    sqlx::postgres::PgSslMode::Require
                } else {
                    sqlx::postgres::PgSslMode::Prefer
                },
            )
    }
}

pub fn get_configuration() -> Result<Settings> {
    let base_path = std::env::current_dir()?;
    let config_dir = base_path.join("configuration");

    let env = std::env::var("APP_ENVIRONMENT")?;
    let config_path = match env.as_str() {
        "PRODUCTION" => config_dir.join("prod.yaml"),
        "LOCAL" => config_dir.join("local.yaml"),
        "CI" => config_dir.join("ci.yaml"),
        _ => bail!(
            "Invalid APP_ENVIRONMENT: {}, use one of PRODUCTION, LOCAL, or CI",
            env
        ),
    };

    let settings = config::Config::builder()
        .add_source(config::File::from(config_dir.join("base.yaml")))
        .add_source(config::File::from(config_path).required(false))
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?;

    Ok(settings.try_deserialize()?)
}
