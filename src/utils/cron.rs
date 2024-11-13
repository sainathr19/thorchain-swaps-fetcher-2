use crate::{db::MySQL, fetcher::fetch_latest_data};

pub async fn start_cronjob(mysql: MySQL) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1800));
    println!("Starting Cronn");
    loop {
        interval.tick().await;
        println!("Fetching Latest Data");
        if let Err(e) = fetch_latest_data(&mysql).await {
            println!("Error pulling latest data: {}", e);
        }
    }
}
