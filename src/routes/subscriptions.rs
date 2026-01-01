use actix_web::{HttpResponse, web};
use anyhow::Result;
use chrono::Utc;
use rand::{Rng, distr::Alphanumeric, rng};
use serde::Deserialize;
use sqlx::{PgPool, Postgres, Transaction};
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::{configuration::Settings, domain::NewSubscriber, email_client::EmailClient};

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
) -> HttpResponse {
    let mut transaction = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            error!("Failed to start DB transaction: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let subscriber: NewSubscriber = match form.0.try_into() {
        Ok(info) => info,
        Err(e) => {
            error!("Failed to parse subscriber info: {}", e);
            return HttpResponse::BadRequest().finish();
        }
    };

    info!("Saving new subscriber details in DB");
    let token = match insert_subscriber(&mut transaction, &subscriber).await {
        Ok(token) => token,
        Err(_) => {
            return HttpResponse::InternalServerError().finish();
        }
    };

    info!("Sending confirmation email");
    let base_url = format!("{}:{}", config.app.base_url, config.app.port);
    if send_confirmation_email(&email_client, &subscriber, &base_url, &token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    match transaction.commit().await {
        Ok(_) => {
            info!("Transaction committed successfully");
            HttpResponse::Ok().finish()
        }
        Err(e) => {
            error!("Failed to commit transaction: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[instrument(name = "Inserting a new subscriber", skip(transaction, subscriber))]
async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber: &NewSubscriber,
) -> Result<String, sqlx::Error> {
    let id = Uuid::new_v4();

    match sqlx::query!(
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
    // deref to get the actual connection as Transaction doesn't
    // implement `Executor` directly
    .execute(&mut **transaction)
    .await
    {
        Ok(_) => {
            info!("Successfully saved customer details");
        }
        Err(e) => {
            error!("Failed to execute query: {:?}", e);
            return Err(e);
        }
    }

    let token = generate_subscription_token();
    store_token(transaction, &id, &token).await?;
    Ok(token)
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

    let res = email_client
        .send_email(&subscriber.email, "Welcome!", &html_body, &text_body)
        .await;

    match res {
        Ok(_) => {
            info!("Confirmation email sent successfully");
            Ok(())
        }
        Err(e) => {
            error!("Failed to send confirmation email: {:?}", e);
            Err(e)
        }
    }
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
) -> Result<(), sqlx::Error> {
    match sqlx::query!(
        r#"
        INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)
        "#,
        token,
        subscriber_id,
    )
    .execute(&mut **transaction)
    .await
    {
        Ok(_) => {
            info!("Subscription token stored successfully");
            Ok(())
        }
        Err(e) => {
            error!("Failed to store subscription token: {:?}", e);
            Err(e)
        }
    }
}
