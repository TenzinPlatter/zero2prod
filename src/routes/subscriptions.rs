use actix_web::{HttpResponse, web};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct FormData {
    _name: String,
    _email: String,
}

pub async fn subscribe(_form: web::Form<FormData>) -> HttpResponse {
    HttpResponse::Ok().finish()
}
