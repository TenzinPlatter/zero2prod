use actix_web::{HttpResponse, web};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::domain::NewSubscriber;

#[derive(Deserialize, Debug)]
pub struct FormData {
    pub name: String,
    pub email: String,
}

#[instrument(
    name = "Adding a new subscriber",
    skip(pool, form)
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    let subscriber = match form.0.try_into() {
        Ok(info) => info,
        Err(e) => {
            error!("Failed to parse subscriber info: {}", e);
            return HttpResponse::BadRequest().finish();
        }
    };

    info!("Saving new subscriber details in DB");
    match insert_subscriber(&pool, &subscriber).await {
        Ok(_) => HttpResponse::Ok().finish(),
        // ignoring the error as it is already logged in insert_subscriber
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[instrument(name = "Inserting a new subscriber", skip(pool, subscriber))]
async fn insert_subscriber(pool: &PgPool, subscriber: &NewSubscriber) -> Result<(), sqlx::Error> {
    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        Uuid::new_v4(),
        subscriber.email.as_ref(),
        subscriber.name.as_ref(),
        Utc::now(),
        "confirmed",
    )
    .execute(pool)
    .await
    {
        Ok(_) => {
            info!("Successfully saved customer details");
            Ok(())
        }
        Err(e) => {
            error!("Failed to execute query: {:?}", e);
            Err(e)
        }
    }
}
