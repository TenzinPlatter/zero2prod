use actix_web::{HttpResponse, web};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{debug, error};
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct FormData {
    name: String,
    email: String,
}

pub async fn subscribe(form: web::Form<FormData>, conn: web::Data<PgPool>) -> HttpResponse {
    debug!("Got query with form: {:?}", form);

    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now(),
    )
    .execute(conn.as_ref())
    .await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            error!("Failed to execute query: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}
