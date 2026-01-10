use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::telemetry::spawn_blocking_with_tracing;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials provided")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, AuthError>;

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

pub struct StoredCredentials {
    pub username: String,
    pub password_hash: Secret<String>,
    pub user_id: Uuid,
}

#[instrument("Validating credentials", skip(pool, creds))]
pub async fn validate_credentials(pool: &PgPool, creds: &Credentials) -> Result<Uuid> {
    let (phc_hash, user_id): (Secret<String>, Option<Uuid>) =
        match get_stored_credentials(pool, &creds.username)
            .await
            .context("Couldn't find stored credentials")
            .map_err(AuthError::InvalidCredentials)?
        {
            Some(creds) => (creds.password_hash, Some(creds.user_id)),
            None => (
                Secret::new(
                    "$argon2id$v=19$m=15000,t=2,p=1$\
                    gZiV/M1gPc22ElAH/Jh1Hw$\
                    CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
                        .to_string(),
                ),
                None,
            ),
        };

    let password = creds.password.clone();

    // ignore result as it returns () if ok, else errors
    match spawn_blocking_with_tracing(move || verify_password_hash(phc_hash, password))
        .await
        .context("Failed to start blocking task")?
    {
        Ok(_) => {
            // this is set to None if user does not exist. Just in case fallback password somehow
            // matches, we still want to return AuthError
            if let Some(user_id) = user_id {
                Ok(user_id)
            } else {
                Err(AuthError::InvalidCredentials(anyhow::anyhow!(
                    "Unknown username"
                )))
            }
        }
        Err(_) => Err(AuthError::InvalidCredentials(anyhow::anyhow!(
            "Invalid password"
        ))),
    }
}

#[instrument("Getting stored credentials", skip(pool))]
async fn get_stored_credentials(
    pool: &PgPool,
    username: &str,
) -> Result<Option<StoredCredentials>> {
    let row = sqlx::query!(
        r#"
        SELECT user_id, username, password_hash FROM users WHERE username = $1
        "#,
        username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to query DB for user credentials")?;

    Ok(row.map(|r| StoredCredentials {
        username: username.to_string(),
        user_id: r.user_id,
        password_hash: Secret::new(r.password_hash),
    }))
}

#[instrument("Verifying password hash", skip(password, phc_hash))]
fn verify_password_hash(phc_hash: Secret<String>, password: Secret<String>) -> Result<()> {
    let expected_hash =
        PasswordHash::new(phc_hash.expose_secret()).context("Failed to parse password hash")?;

    Argon2::default()
        .verify_password(password.expose_secret().as_bytes(), &expected_hash)
        .context("Invalid password")
        .map_err(AuthError::InvalidCredentials)
}
