use crate::{models::actions_model::ActionsFetchResponse, RATE_LIMIT_DELAY, REQUEST_SEMAPHORE};
use reqwest::Client;
use std::time::Duration;

pub struct MidGard;
impl MidGard {
    async fn fetch_with_retry(
        url: &str,
        client: &Client,
    ) -> Result<ActionsFetchResponse, reqwest::Error> {
        let mut attempts = 0;
        let max_attempts = 10;

        loop {
            attempts += 1;
            println!("Fetching URL (Attempt {}): {}", attempts, url);

            let _permit = REQUEST_SEMAPHORE.acquire().await.unwrap();
            tokio::time::sleep(*RATE_LIMIT_DELAY).await;

            let response = client.get(url).send().await;

            match response {
                Ok(resp) => {
                    let parsed_response = resp.json::<ActionsFetchResponse>().await;
                    match parsed_response {
                        Ok(data) => return Ok(data),
                        Err(e) => {
                            println!("Failed to parse response: {:?}", e);
                            if attempts >= max_attempts {
                                return Err(e);
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("Request failed: {:?}", e);
                    if attempts >= max_attempts {
                        return Err(e);
                    }
                }
            }
        }
    }

    pub async fn fetch_actions_with_nextpage(
        base_url: &str,
        next_page_token: &str,
    ) -> Result<ActionsFetchResponse, reqwest::Error> {
        let client = Client::builder().timeout(Duration::from_secs(15)).build()?;
        let url = if next_page_token.is_empty() {
            base_url.to_string()
        } else {
            format!(
                "{}&nextPageToken={}",
                base_url,next_page_token
            )
        };
        Self::fetch_with_retry(&url, &client).await
    }

    pub async fn fetch_actions_with_prevpage(
        base_url: &str,
        prev_page_token: &str,
    ) -> Result<ActionsFetchResponse, reqwest::Error> {
        let client = Client::builder().timeout(Duration::from_secs(15)).build()?;
        let url = format!(
            "{}&prevPageToken={}",
            base_url,
            prev_page_token
        );
        Self::fetch_with_retry(&url, &client).await
    }

    pub async fn fetch_actions_with_timestamp(
        base_url: &str,
        timestamp: &str,
    ) -> Result<ActionsFetchResponse, reqwest::Error> {
        let client = Client::builder().timeout(Duration::from_secs(15)).build()?;
        let url = format!(
            "{}&fromTimestamp={}",
            base_url,
            timestamp
        );
        Self::fetch_with_retry(&url, &client).await
    }

    pub async fn fetch_action_with_transactionid(
        base_url: &str,
        tx_id: String,
    ) -> Result<ActionsFetchResponse, reqwest::Error> {
        let client = Client::builder().timeout(Duration::from_secs(15)).build()?;
        let url = format!(
            "{}?txid={}",
            base_url,
            tx_id
        );
        Self::fetch_with_retry(&url, &client).await
    }
}
