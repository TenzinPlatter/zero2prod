use actix_web::HttpResponse;
use tracing::instrument;

#[instrument()]
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}
