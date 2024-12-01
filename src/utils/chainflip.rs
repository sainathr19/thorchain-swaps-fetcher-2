use crate::models::chainflip_swaps::SwapResponse;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

pub struct ChainFlip;

impl ChainFlip {
    async fn fetch_with_retry(
        url: &str,
        client: &Client,
        cursor: Option<&str>,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<SwapResponse, Box<dyn std::error::Error>> {
        let mut attempts = 0;
        let max_attempts = 10;

        loop {
            attempts += 1;
            println!("Fetching URL (Attempt {}): {}", attempts, url);

            let body = json!({
                "query": query,
                "variables": variables
            });

            match client.post(url)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
            {
                Ok(resp) => {
                    let text = resp.text().await?;                    
                    match serde_json::from_str::<SwapResponse>(&text) {
                        Ok(data) => return Ok(data),
                        Err(e) => {
                            println!("Parse error: {:?}", e);
                            if attempts >= max_attempts {
                                return Err(Box::new(e));
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("Request error: {:?}", e);
                    if attempts >= max_attempts {
                        return Err(Box::new(e));
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    pub async fn fetch_chainflip_swaps(
        base_url: &str,
        first: Option<i32>,
        offset: Option<i32>,
        destination_address: Option<&str>,
    ) -> Result<SwapResponse, Box<dyn std::error::Error>> {
        let client = Client::builder().timeout(Duration::from_secs(15)).build()?;
        
        let query = r#"
            query GetAllSwaps($first: Int, $offset: Int, $destinationAddress: String) {
                allSwapRequests(
                    orderBy: NATIVE_ID_DESC
                    offset: $offset
                    first: $first
                    filter: {
                        destinationAddress: {includesInsensitive: $destinationAddress}, 
                        type: {in: [REGULAR, CCM, LEGACY_SWAP]}
                    }
                ) {
                    pageInfo {
                        hasPreviousPage
                        startCursor
                        hasNextPage
                        endCursor
                    }
                    edges {
                        node {
                            id
                            nativeId
                            sourceAsset
                            depositAmount
                            depositValueUsd
                            destinationAsset
                            destinationAddress
                            type
                            egressId
                            egressByEgressId {
                                amount
                                valueUsd
                                eventByScheduledEventId {
                                    blockByBlockId {
                                        timestamp
                                    }
                                }
                            }
                            executedSwaps: swapsBySwapRequestId(
                                filter: {swapExecutedEventId: {isNull: false}, type: {notEqualTo: GAS}}
                            ) {
                                totalCount
                                aggregates {
                                    sum {
                                        swapInputAmount
                                        swapInputValueUsd
                                        intermediateAmount
                                        intermediateValueUsd
                                        swapOutputAmount
                                        swapOutputValueUsd
                                    }
                                }
                            }
                            swapChannelByDepositChannelId {
                                sourceAsset
                                depositAddress
                                destinationAsset
                                destinationAddress
                            }
                        }
                    }
                    totalCount
                }
            }
        "#;

        let variables = json!({
            "first": first.unwrap_or(20),
            "offset": offset.unwrap_or(0),
            "destinationAddress": destination_address
        });

        Self::fetch_with_retry(base_url, &client, None, query, variables).await
    }

    pub async fn fetch_single_swap(
        base_url: &str,
        native_id: i64,
    ) -> Result<SwapResponse, Box<dyn std::error::Error>> {
        let client = Client::builder().timeout(Duration::from_secs(15)).build()?;
        
        let query = r#"
            query GetSwapByNativeId($nativeId: BigInt!) {
                swapRequest: swapRequestByNativeId(nativeId: $nativeId) {
                    id
                    nativeId
                    sourceAsset
                    depositAmount
                    depositValueUsd
                    destinationAsset
                    destinationAddress
                    type
                    egress {
                        amount
                        valueUsd
                    }
                    channel {
                        sourceAsset
                        depositAddress
                        destinationAsset
                        destinationAddress
                    }
                }
            }
        "#;

        let variables = json!({
            "nativeId": native_id
        });

        Self::fetch_with_retry(base_url, &client, None, query, variables).await
    }
}
