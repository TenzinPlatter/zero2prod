use actix_web::{HttpResponse, ResponseError, web};
use anyhow::{Context, Result};
use sqlx::PgPool;
use tracing::{error, info};

use crate::telemetry::error_chain_fmt;

#[derive(thiserror::Error)]
pub enum SubscriptionConfirmationError {
    #[error("There is no subscriber associated with the provided token.")]
    UnknownToken,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for SubscriptionConfirmationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for SubscriptionConfirmationError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            SubscriptionConfirmationError::UnknownToken => {
                actix_web::http::StatusCode::UNAUTHORIZED
            }
            SubscriptionConfirmationError::UnexpectedError(_) => {
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

#[derive(serde::Deserialize)]
pub struct ConfirmSubscriptionParams {
    pub subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscription", skip(parameters))]
pub async fn confirm_subscription(
    parameters: web::Query<ConfirmSubscriptionParams>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, SubscriptionConfirmationError> {
    let id = get_subscriber_id_from_token(pool.get_ref(), &parameters.subscription_token)
        .await
        .context("Failed to get subscriber id from token")?
        .ok_or(SubscriptionConfirmationError::UnknownToken)
        .context("No user found for given token")?;

    confirm_subscription_in_db(&pool, id).await?;
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Fetching subscriber id from token", skip(pool))]
async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<uuid::Uuid>, sqlx::Error> {
    Ok(sqlx::query!(
        r#"
            SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1
            "#,
        subscription_token,
    )
    .fetch_optional(pool)
    .await?
    .map(|record| record.subscriber_id))
}

#[tracing::instrument(name = "Marking subscriber as confirmed", skip(pool))]
async fn confirm_subscription_in_db(pool: &PgPool, id: uuid::Uuid) -> Result<()> {
    match sqlx::query!(
        r#"
        UPDATE subscriptions SET status = 'confirmed' WHERE id = $1
        "#,
        id,
    )
    .execute(pool)
    .await
    {
        Ok(_) => {
            info!("Subscriber status updated to confirmed");
            Ok(())
        }
        Err(e) => {
            error!("Failed to update subscription status: {}", e);
            Err(e.into())
        }
    }
}
