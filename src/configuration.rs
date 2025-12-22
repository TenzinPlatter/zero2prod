use serde::Deserialize;

#[derive(Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application_port: u16,
}

#[derive(Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: String,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database_name
        )
    }

    pub fn postgres_connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/postgres",
            self.username, self.password, self.host, self.port
        )
    }
}

pub fn get_configuration<F>(apply_overrides: F) -> Result<Settings, config::ConfigError>
where
    F: Fn(Settings) -> Settings,
{
    let config_path = std::env::var("CONFIG_FILE")
        .ok()
        .unwrap_or("configuration.yaml".to_string());

    let settings = config::Config::builder()
        .add_source(config::File::new(&config_path, config::FileFormat::Yaml).required(false))
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?;

    let settings = settings.try_deserialize::<Settings>()?;
    Ok(apply_overrides(settings))
}
