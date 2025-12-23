use anyhow::{Result, bail};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub app: ApplicationSettings,
}

#[derive(Deserialize)]
pub struct ApplicationSettings {
    pub port: u16,
    pub host: String,
}

#[derive(Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub max_connections: u8,
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> Secret<String> {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database_name
        )
        .into()
    }

    pub fn postgres_connection_string(&self) -> Secret<String> {
        format!(
            "postgres://{}:{}@{}:{}/postgres",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port
        )
        .into()
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

    Ok(settings.try_deserialize::<Settings>()?)
}
