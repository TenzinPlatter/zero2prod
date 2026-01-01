use actix_web::{HttpResponse, web};
use anyhow::Result;
use sqlx::PgPool;
use tracing::{error, info};

#[derive(serde::Deserialize)]
pub struct ConfirmSubscriptionParams {
    pub subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscription", skip(parameters))]
pub async fn confirm_subscription(
    parameters: web::Query<ConfirmSubscriptionParams>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let id =
        match get_subscriber_id_from_token(pool.get_ref(), &parameters.subscription_token).await {
            Err(_) => return HttpResponse::InternalServerError().finish(),
            Ok(None) => {
                info!("No subscriber found for the provided token");
                return HttpResponse::Unauthorized().finish();
            }
            Ok(Some(subscriber_id)) => subscriber_id,
        };

    if confirm_subscription_in_db(&pool, id).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    };

    HttpResponse::Ok().finish()
}

#[tracing::instrument(name = "Fetching subscriber id from token", skip(pool))]
async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<uuid::Uuid>, sqlx::Error> {
    match sqlx::query!(
        r#"
        SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1
        "#,
        subscription_token,
    )
    .fetch_optional(pool)
    .await
    {
        Err(e) => {
            error!("Failed to retrieve subscription token: {}", e);
            Err(e)
        }
        Ok(id) => {
            info!("Subscription token found, returning subscriber ID");
            Ok(id.map(|record| record.subscriber_id))
        }
    }
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
