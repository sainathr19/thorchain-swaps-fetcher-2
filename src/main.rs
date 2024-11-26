mod db;
mod fetcher;
mod models;
mod routes;
mod tests;
mod utils;
use actix_cors::Cors;
use actix_web::{get, web::Data, App, HttpResponse, HttpServer, Responder};
use db::PostgreSQL;
use utils::cron::{start_cronjob, start_retry};

#[get("/")]
async fn home() -> impl Responder {
    HttpResponse::Ok().body("Rust Backend Server")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {    
    let pg = PostgreSQL::init().await.expect("Error COnnecting to POSTGRESQL");
    
    let pg_clone = pg.clone();
    tokio::spawn(async move { start_cronjob(pg_clone).await });

    let pg_clone = pg.clone();
    tokio::spawn(async move { start_retry(pg_clone).await });

    let pg_data = Data::new(pg);
    let server = HttpServer::new(move || {
        App::new()
            .app_data(pg_data.clone())
            .wrap(Cors::permissive())
            .service(home)
            .configure(routes::swap_history::init)
    })
    .bind(("0.0.0.0", 3000))
    .expect("Failed to bind Actix server")
    .run();

    server.await?;
    Ok(())
}
