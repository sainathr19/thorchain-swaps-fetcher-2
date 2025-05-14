mod db;
mod fetcher;
mod models;
mod routes;
mod tests;
mod utils;
use std::{collections::HashSet, sync::Arc, time::Duration};

use actix_cors::Cors;
use actix_web::{get, web::Data, App, HttpResponse, HttpServer, Responder};
use db::PostgreSQL;
use futures_util::lock::Mutex;
use lazy_static::lazy_static;
use tokio::sync::Semaphore;
use utils::cron::{
    start_chainflip_swaps_incremental, start_cronjob, start_daily_fetch, start_fetch_closing_price,
    start_retry,
};

#[get("/")]
async fn home() -> impl Responder {
    HttpResponse::Ok().body("Rust Backend Server")
}

const NATIVE_SWAPS_BASE_URL: &str =
    "https://vanaheimex.com/actions?asset=notrade,BTC.BTC&type=swap";
const TRADE_SWAPS_BASE_URL: &str = "https://vanaheimex.com/actions?asset=trade,BTC~BTC&type=swap";
const CHAINFLIP_BASE_URL: &str = "https://reporting-service.chainflip.io/graphql";

lazy_static! {
    pub static ref TRADE_SWAPS_PENDING_IDS: Arc<Mutex<HashSet<String>>> =
        Arc::new(Mutex::new(HashSet::new()));
}
lazy_static! {
    pub static ref NATIVE_SWAPS_PENDING_IDS: Arc<Mutex<HashSet<String>>> =
        Arc::new(Mutex::new(HashSet::new()));
}

lazy_static::lazy_static! {
    static ref REQUEST_SEMAPHORE: Arc<Semaphore> = Arc::new(Semaphore::new(1));
    static ref RATE_LIMIT_DELAY: Duration = Duration::from_millis(5000);
}

#[derive(Debug, PartialEq, Clone)]
pub enum SwapType {
    NATIVE,
    TRADE,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let pg = PostgreSQL::init()
        .await
        .expect("Error Connecting to POSTGRESQL");

    tokio::spawn({
        let pg = pg.clone();
        async move { start_cronjob(pg.clone(), NATIVE_SWAPS_BASE_URL, SwapType::NATIVE).await }
    });

    tokio::spawn({
        let pg = pg.clone();
        async move {
            start_retry(
                pg.clone(),
                NATIVE_SWAPS_BASE_URL,
                NATIVE_SWAPS_PENDING_IDS.clone(),
                SwapType::NATIVE,
            )
            .await
        }
    });

    tokio::spawn({
        let pg = pg.clone();
        async move { start_cronjob(pg.clone(), TRADE_SWAPS_BASE_URL, SwapType::TRADE).await }
    });

    tokio::spawn({
        let pg = pg.clone();
        async move {
            start_retry(
                pg.clone(),
                TRADE_SWAPS_BASE_URL,
                TRADE_SWAPS_PENDING_IDS.clone(),
                SwapType::TRADE,
            )
            .await
        }
    });

    tokio::spawn({
        let pg = pg.clone();
        async move { start_fetch_closing_price(pg.clone()).await }
    });

    tokio::spawn({
        let pg = pg.clone();
        async move { start_chainflip_swaps_incremental(pg.clone(), CHAINFLIP_BASE_URL).await }
    });

    tokio::spawn({
        let pg = pg.clone();
        async move { start_daily_fetch(pg.clone()).await }
    });

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
