mod db;
mod fetcher;
mod models;
mod routes;
mod tests;
mod utils;
use std::{collections::HashSet, sync::Arc};

use actix_cors::Cors;
use actix_web::{get, web::Data, App, HttpResponse, HttpServer, Responder};
use db::PostgreSQL;
use fetcher::fetch_historical_data;
use futures_util::lock::Mutex;
use lazy_static::lazy_static;
use utils::cron::{start_cronjob, start_retry};

#[get("/")]
async fn home() -> impl Responder {
    HttpResponse::Ok().body("Rust Backend Server")
}

const NATIVE_SWAPS_BASE_URL: &str = "https://vanaheimex.com/actions?limit=30&asset=notrade,BTC.BTC&txType=swap&type=swap";
const TRADE_SWAPS_BASE_URL: &str = "https://vanaheimex.com/actions?limit=30&asset=trade,BTC~BTC&type=swap";

lazy_static! {
    pub static ref TRADE_SWAPS_PENDING_IDS: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
}
lazy_static! {
    pub static ref NATIVE_SWAPS_PENDING_IDS: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
}

#[derive(Debug, PartialEq, Clone)]
pub enum SwapType {
    NATIVE,
    TRADE,
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {    
    let pg = PostgreSQL::init().await.expect("Error COnnecting to POSTGRESQL");

    // tokio::spawn(async move{ fetch_historical_data().await});
    
    let pg_clone = pg.clone();
    tokio::spawn(async move { start_cronjob(pg_clone,NATIVE_SWAPS_BASE_URL,SwapType::NATIVE).await });
    let pg_clone = pg.clone();
    tokio::spawn(async move { start_retry(pg_clone,NATIVE_SWAPS_BASE_URL,NATIVE_SWAPS_PENDING_IDS.clone(),SwapType::NATIVE).await });

    let pg_clone = pg.clone();
    tokio::spawn(async move { start_cronjob(pg_clone,TRADE_SWAPS_BASE_URL,SwapType::TRADE).await });
    let pg_clone = pg.clone();
    tokio::spawn(async move { start_retry(pg_clone,TRADE_SWAPS_BASE_URL,TRADE_SWAPS_PENDING_IDS.clone(),SwapType::TRADE).await });



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
