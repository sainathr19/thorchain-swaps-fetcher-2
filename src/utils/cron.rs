use crate::{db::PostgreSQL, fetcher::{fetch_latest_data, retry_pending_transactions}};

pub async fn start_cronjob(pg: PostgreSQL) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600));
    loop {
        interval.tick().await;
        println!("Fetching Latest Data");
        if let Err(e) = fetch_latest_data(&pg).await {
            println!("Error pulling latest data: {}", e);
        }
    }
}

pub async fn start_retry(pg: PostgreSQL) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600));
    loop {
        interval.tick().await;
        println!("Retrying Pending Transactions");
        if let Err(e) = retry_pending_transactions(&pg).await {
            println!("Error pulling latest data: {}", e);
        }
    }
}
