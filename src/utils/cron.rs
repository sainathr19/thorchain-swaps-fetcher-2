use std::{collections::HashSet, sync::Arc};

use futures_util::lock::Mutex;

use crate::{db::PostgreSQL, fetcher::{fetch_btc_closing_price, fetch_chainflip_swaps, fetch_latest_data, retry_pending_transactions}, SwapType};


pub async fn start_cronjob(pg: PostgreSQL,base_url: &str,swap_type: SwapType) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
    loop {
        interval.tick().await;
        let swap_type_str = match swap_type {
            SwapType::NATIVE => "Native Swaps",
            SwapType::TRADE => "Trade Swaps"
        };
        println!("Fetching Latest {} Data", swap_type_str);
        if let Err(e) = fetch_latest_data(&pg,base_url,swap_type.clone()).await {
            println!("Error pulling latest {} data: {}", swap_type_str, e);
        }
    }
}

pub async fn start_retry(pg: PostgreSQL,base_url: &str,pending_ids: Arc<Mutex<HashSet<String>>>,swap_type: SwapType) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
    loop {
        interval.tick().await;
        let swap_type_str = match swap_type {
            SwapType::NATIVE => "Native Swaps",
            SwapType::TRADE => "Trade Swaps"
        };
        println!("Retrying Pending {} Transactions", swap_type_str);
        if let Err(e) = retry_pending_transactions(&pg,base_url,pending_ids.clone(),swap_type.clone()).await {
            println!("Error retrying pending {} transactions: {}", swap_type_str, e);
        }
    }
}

pub async fn start_fetch_closing_price(pg: PostgreSQL) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(86400)); // 24 hours
    loop {
        interval.tick().await;
        println!("Fetching BTC Closing Price");
        if let Err(e) = fetch_btc_closing_price(&pg).await {
            println!("Error fetching closing price: {}", e);
        }
    }
}


pub async fn start_fetch_chainflip_swaps(pg: PostgreSQL,base_url: &str) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
    loop {
        println!("Fetching Chainflip Swaps");
        if let Err(e) = fetch_chainflip_swaps(&base_url,&pg).await {
            println!("Error fetching chainflip swaps: {}", e);
        }
        interval.tick().await;
    }
}

