use crate::models::actions_model::ActionsFetchResponse;
use reqwest::Client;
use std::time::Duration;

pub struct MidGard;

impl MidGard {
    async fn fetch_with_retry(
        url: &str,
        client: &Client,
    ) -> Result<ActionsFetchResponse, reqwest::Error> {
        let mut attempts = 0;
        let max_attempts = 3;

        loop {
            attempts += 1;
            println!("Fetching URL (Attempt {}): {}", attempts, url);
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
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    pub async fn fetch_actions_with_nextpage(
        next_page_token: &str,
    ) -> Result<ActionsFetchResponse, reqwest::Error> {
        let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
        let url = if next_page_token.is_empty() {
            "https://vanaheimex.com/actions?type=swap&asset=notrade".to_string()
        } else {
            format!(
                "https://vanaheimex.com/actions?type=swap&asset=notrade&nextPageToken={}",
                next_page_token
            )
        };
        Self::fetch_with_retry(&url, &client).await
    }

    pub async fn fetch_actions_with_prevpage(
        prev_page_token: &str,
    ) -> Result<ActionsFetchResponse, reqwest::Error> {
        let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
        let url = format!(
            "https://vanaheimex.com/actions?type=swap&asset=notrade&prevPageToken={}",
            prev_page_token
        );
        Self::fetch_with_retry(&url, &client).await
    }

    pub async fn fetch_actions_with_timestamp(
        timestamp: &str,
    ) -> Result<ActionsFetchResponse, reqwest::Error> {
        let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
        let url = format!(
            "https://vanaheimex.com/actions?type=swap&asset=notrade&fromTimestamp={}",
            timestamp
        );
        Self::fetch_with_retry(&url, &client).await
    }
}
