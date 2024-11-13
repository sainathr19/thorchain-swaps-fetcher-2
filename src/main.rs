mod db;
mod fetcher;
mod models;
mod routes;
mod tests;
mod utils;
use actix_cors::Cors;
use actix_web::{get, web::Data, App, HttpResponse, HttpServer, Responder};
use db::MySQL;
use fetcher::fetch_historical_data;
use utils::cron::start_cronjob;

#[get("/")]
async fn home() -> impl Responder {
    HttpResponse::Ok().body("Rust Backend Server")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tokio::spawn(async move { fetch_historical_data().await });

    let mysql = MySQL::init().await.expect("Error COnnecting to SQL");
    let mysql_clone = mysql.clone();
    tokio::spawn(async move { start_cronjob(mysql_clone).await });

    // Create mysql_data for the Actix app
    let mysql_data = Data::new(mysql);
    let server = HttpServer::new(move || {
        App::new()
            .app_data(mysql_data.clone())
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
