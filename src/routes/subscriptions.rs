use std::fmt::Debug;

use actix_web::{HttpResponse, ResponseError, web};
use anyhow::{Context, Result};
use chrono::Utc;
use rand::{Rng, distr::Alphanumeric, rng};
use serde::Deserialize;
use sqlx::{PgPool, Postgres, Transaction};
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::{
    configuration::Settings, domain::NewSubscriber, email_client::EmailClient,
    telemetry::error_chain_fmt,
};

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl From<String> for SubscribeError {
    fn from(value: String) -> Self {
        SubscribeError::ValidationError(value)
    }
}

impl Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            SubscribeError::ValidationError(_) => actix_web::http::StatusCode::BAD_REQUEST,
            SubscribeError::UnexpectedError(_) => {
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct FormData {
    pub name: String,
    pub email: String,
}

#[instrument(
    name = "Adding a new subscriber",
    skip(pool, form, email_client, config)
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    config: web::Data<Settings>,
) -> Result<HttpResponse, SubscribeError> {
    let mut transaction = pool
        .begin()
        .await
        .context("Failed to begin SQL transaction")?;

    let subscriber: NewSubscriber = form.0.try_into()?;

    info!("Saving new subscriber details in DB");
    let token = generate_subscription_token();
    let id = insert_subscriber(&mut transaction, &subscriber)
        .await
        .context("Failed to insert new subscriber")?;
    store_token(&mut transaction, &id, &token)
        .await
        .context("Failed to store subscription token")?;

    info!("Sending confirmation email");
    let base_url = format!("{}:{}", config.app.base_url, config.app.port);
    send_confirmation_email(&email_client, &subscriber, &base_url, &token).await?;
    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction")?;

    Ok(HttpResponse::Ok().finish())
}

#[instrument(name = "Inserting a new subscriber", skip(transaction, subscriber))]
async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        id,
        subscriber.email.as_ref(),
        subscriber.name.as_ref(),
        Utc::now(),
        "pending_confirmation",
    )
    .execute(&mut **transaction)
    .await?;

    Ok(id)
}

#[instrument(
    name = "Sending confirmation email",
    skip(email_client, subscriber, base_url, token)
)]
async fn send_confirmation_email(
    email_client: &EmailClient,
    subscriber: &NewSubscriber,
    base_url: &str,
    token: &str,
) -> Result<()> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, token,
    );

    let text_body = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );

    let html_body = format!(
        "Welcome to our newsletter!<br />\
                Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );

    email_client
        .send_email(&subscriber.email, "Welcome!", &html_body, &text_body)
        .await?;

    Ok(())
}

#[instrument(name = "Generating subscription token")]
fn generate_subscription_token() -> String {
    let mut rng = rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .take(25)
        .map(char::from)
        .collect()
}

#[instrument(
    name = "Storing subscription token",
    skip(transaction, subscriber_id, token)
)]
async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: &Uuid,
    token: &str,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)
        "#,
        token,
        subscriber_id,
    )
    .execute(&mut **transaction)
    .await?;

    Ok(())
}
