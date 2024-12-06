use std::{collections::HashSet, sync::Arc};
use chrono::{DateTime, Duration, NaiveTime, Utc};
use futures_util::lock::Mutex;

use crate::{db::PostgreSQL, fetcher::{fetch_btc_closing_price, fetch_chainflip_swaps, fetch_daily_data, fetch_latest_data, retry_pending_transactions}, SwapType, NATIVE_SWAPS_BASE_URL, TRADE_SWAPS_BASE_URL};


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
    loop {
        let now: DateTime<Utc> = Utc::now();
        let next_run = {
            let target_time = NaiveTime::from_hms_opt(0, 5, 0).unwrap();
            if now.time() >= target_time {
                (now.date_naive() + Duration::days(1)).and_time(target_time)
            } else {
                now.date_naive().and_time(target_time)
            }
        }.and_utc();

        let delay = next_run - now;
        tokio::time::sleep(delay.to_std().unwrap()).await;

        println!("Fetching BTC Price");
        if let Err(e) = fetch_btc_closing_price(&pg).await {
            println!("Error fetching closing price: {}", e);
        }
    }
}


pub async fn start_fetch_chainflip_swaps(pg: PostgreSQL,base_url: &str) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(900));
    loop {
        interval.tick().await;
        println!("Fetching Chainflip Swaps");
        if let Err(e) = fetch_chainflip_swaps(&base_url,&pg).await {
            println!("Error fetching chainflip swaps: {}", e);
        }
    }
}

pub async fn start_daily_fetch(pg: PostgreSQL) {
    loop {
        let now: DateTime<Utc> = Utc::now();
        let next_run = {
            let current_time = now.time();
            let morning = NaiveTime::from_hms_opt(11, 55, 0).unwrap();
            let evening = NaiveTime::from_hms_opt(23, 55, 0).unwrap();
            
            if current_time < morning {
                now.date_naive().and_time(morning)
            } else if current_time < evening {
                now.date_naive().and_time(evening)
            } else {
                (now.date_naive() + Duration::days(1)).and_time(morning)
            }
        }.and_utc();

        let delay = next_run - now;
        tokio::time::sleep(delay.to_std().unwrap()).await;

        let start_of_period = now.date_naive().and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        let epoch_timestamp = start_of_period.and_utc().timestamp();
        
        println!("Running reconcile fetch job with epoch: {}", epoch_timestamp);
        if let Err(e) = fetch_daily_data(&pg, &NATIVE_SWAPS_BASE_URL, SwapType::NATIVE, epoch_timestamp).await {
            println!("Error in reconcile fetch job: {}", e);
        }
    }
}

