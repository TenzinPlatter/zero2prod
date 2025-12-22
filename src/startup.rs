use actix_web::{App, HttpServer, dev::Server, web};
use anyhow::Result;
use sqlx::{PgPool};

use std::net::TcpListener;

use crate::routes::{health_check, subscribe};

pub fn run(listener: TcpListener, connection: PgPool) -> Result<Server> {
    let connection = web::Data::new(connection);
    Ok(HttpServer::new(move || {
        App::new()
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(connection.clone())
    })
    .listen(listener)?
    .run())
}
