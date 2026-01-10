use actix_web::{HttpResponse, error::InternalError, http::header::LOCATION, web};
use actix_web_flash_messages::FlashMessage;
use secrecy::Secret;
use sqlx::PgPool;

use crate::{
    auth::{AuthError, Credentials, validate_credentials},
    routes::LoginError,
};

#[derive(serde::Deserialize)]
pub struct FormData {
    pub username: String,
    pub password: String,
}

#[tracing::instrument(name = "Handling login form submission", skip(form, pool))]
pub async fn login_post(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, InternalError<String>> {
    match validate_credentials(
        &pool,
        &Credentials {
            username: form.username.clone(),
            password: Secret::new(form.password.clone()),
        },
    )
    .await
    {
        Ok(_) => Ok(HttpResponse::SeeOther()
            .insert_header((LOCATION, "/"))
            .finish()),
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(e) => LoginError::AuthError(e),
                AuthError::UnexpectedError(e) => LoginError::UnexpectedError(e),
            };

            FlashMessage::error(e.to_string()).send();
            let response = HttpResponse::SeeOther()
                .insert_header((LOCATION, "/login"))
                .finish();

            Err(InternalError::from_response(e.to_string(), response))
        }
    }
}
