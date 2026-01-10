use actix_web::{HttpResponse, ResponseError, http::header::ContentType};
use actix_web_flash_messages::{IncomingFlashMessages, Level};
pub use post::login_post;
use tracing::instrument;

pub mod post;

#[derive(thiserror::Error, Debug)]
pub enum LoginError {
    #[error("Authentication error")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for LoginError {
    fn error_response(&self) -> HttpResponse {
        match self {
            LoginError::AuthError(_) => HttpResponse::Unauthorized().finish(),
            LoginError::UnexpectedError(_) => {
                HttpResponse::InternalServerError().body("Internal server error")
            }
        }
    }
}

#[instrument(name = "Rendering login form", skip(flash_messages))]
pub async fn login_form(flash_messages: IncomingFlashMessages) -> Result<HttpResponse, LoginError> {
    let messages_count = flash_messages.iter().count();
    tracing::info!("flash messages: {}", messages_count);
    let error_html = flash_messages
        .iter()
        .filter(|m| m.level() == Level::Error)
        .fold(String::new(), |mut acc, m| {
            acc.push_str(&format!(r#"<p><i>{}</i></p>"#, m.content()));
            acc
        });

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(include_str!("./login.html"), error_html)))
}
