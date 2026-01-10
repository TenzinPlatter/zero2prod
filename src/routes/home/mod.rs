use actix_web::{HttpResponse, ResponseError, http::header::ContentType};

#[derive(thiserror::Error, Debug)]
pub enum HomeError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for HomeError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            HomeError::UnexpectedError(_) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub type Result<T> = std::result::Result<T, HomeError>;

pub async fn home() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(include_str!("./home.html")))
}
