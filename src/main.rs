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
use futures_util::lock::Mutex;
use lazy_static::lazy_static;
use utils::cron::{start_cronjob, start_daily_fetch, start_fetch_chainflip_swaps, fetch_daily_closing_price, start_retry};

#[get("/")]
async fn home() -> impl Responder {
    HttpResponse::Ok().body("Rust Backend Server")
}

const NATIVE_SWAPS_BASE_URL: &str = "https://vanaheimex.com/actions?asset=notrade,BTC.BTC&type=swap";
const TRADE_SWAPS_BASE_URL: &str = "https://vanaheimex.com/actions?asset=trade,BTC~BTC&type=swap";
const CHAINFLIP_BASE_URL: &str = "https://explorer-service-processor.chainflip.io/graphql";

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
    
    let pg = Arc::new(pg);

    // FETCH NATIVE SWAPS
    tokio::spawn({
        let pg = pg.clone();
        async move { start_cronjob((*pg).clone(), NATIVE_SWAPS_BASE_URL, SwapType::NATIVE).await }
    });

    // RETRY PENDING NATIVE SWAPS
    tokio::spawn({
        let pg = pg.clone();
        async move { start_retry((*pg).clone(), NATIVE_SWAPS_BASE_URL, NATIVE_SWAPS_PENDING_IDS.clone(), SwapType::NATIVE).await }
    });

    // FETCH TRADE SWAPS
    tokio::spawn({
        let pg = pg.clone();
        async move { start_cronjob((*pg).clone(), TRADE_SWAPS_BASE_URL, SwapType::TRADE).await }
    });

    // RETRY PENDING TRADE SWAPS
    tokio::spawn({
        let pg = pg.clone();
        async move { start_retry((*pg).clone(), TRADE_SWAPS_BASE_URL, TRADE_SWAPS_PENDING_IDS.clone(), SwapType::TRADE).await }
    });

    // FETCH DAILY CLOSING PRICE
    tokio::spawn({
        let pg = pg.clone();
        async move { fetch_daily_closing_price((*pg).clone()).await }
    });

    // FETCH CHAINFLIP SWAPS
    tokio::spawn({
        let pg = pg.clone();
        async move { start_fetch_chainflip_swaps((*pg).clone(),CHAINFLIP_BASE_URL).await }
    });

    // FETCH DAILY DATA
    tokio::spawn({
        let pg = pg.clone();
        async move { start_daily_fetch((*pg).clone()).await }
    });

    let pg_data = Data::new((*pg).clone());
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
