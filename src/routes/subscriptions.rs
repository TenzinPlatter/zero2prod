use actix_web::{HttpResponse, web};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{error, info, instrument};
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct FormData {
    name: String,
    email: String,
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
    info!("Saving new subscriber details in DB");
    match insert_subscriber(&pool, &form.email, &form.name).await {
        Ok(_) => HttpResponse::Ok().finish(),
        // ignoring the error as it is already logged in insert_subscriber
        Err(_e) => HttpResponse::InternalServerError().finish(),
    }
}

#[instrument(name = "Inserting a new subscriber", skip(pool, email, name))]
async fn insert_subscriber(pool: &PgPool, email: &str, name: &str) -> Result<(), sqlx::Error> {
    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        email,
        name,
        Utc::now(),
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
