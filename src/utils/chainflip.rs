use crate::models::chainflip_swaps::SwapResponse;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

pub struct ChainFlip;

impl ChainFlip {
    async fn fetch_with_retry(
        url: &str,
        client: &Client,
        _cursor: Option<&str>,
        query: &str,
        variables: serde_json::Value,
        operation_name: &str,
    ) -> Result<SwapResponse, Box<dyn std::error::Error + Send + Sync>> {
        let mut attempts = 0;
        let max_attempts = 10;

        loop {
            attempts += 1;
            println!("Fetching URL (Attempt {}): {}", attempts, url);

            let body = json!({
                "query": query,
                "variables": variables,
                "operationName": operation_name
            });

            match client
                .post(url)
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
                            println!("Response text: {}", text);
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
    ) -> Result<SwapResponse, Box<dyn std::error::Error + Send + Sync>> {
        let client = Client::builder().timeout(Duration::from_secs(15)).build()?;

        let query = r#"
            query GetAllSwaps($first: Int, $offset: Int, $destinationOrRefundAddress: String, $swapRequestNativeId: BigInt, $mainBrokerAccountSs58Id: String, $affiliateBrokerAccountSs58Id: String, $asset: ChainflipAsset, $isOnChain: Boolean, $lpRefundAddress: String, $alias: String) {
                allSwapRequests(
                    orderBy: [IS_IN_PROGRESS_DESC, SWAP_REQUEST_NATIVE_ID_DESC]
                    offset: $offset
                    first: $first
                    condition: {isOnChain: $isOnChain, destinationAddress: $lpRefundAddress, refundAddress: $lpRefundAddress}
                    filter: {or: [{destinationAddress: {includesInsensitive: $destinationOrRefundAddress}}, {refundAddress: {includesInsensitive: $destinationOrRefundAddress}}, {swapRequestNativeId: {equalTo: $swapRequestNativeId}}, {mainBrokerAccountSs58Id: {equalTo: $mainBrokerAccountSs58Id}}, {affiliateBroker1AccountSs58Id: {equalTo: $affiliateBrokerAccountSs58Id}}, {affiliateBroker2AccountSs58Id: {equalTo: $affiliateBrokerAccountSs58Id}}, {affiliateBroker3AccountSs58Id: {equalTo: $affiliateBrokerAccountSs58Id}}, {affiliateBroker4AccountSs58Id: {equalTo: $affiliateBrokerAccountSs58Id}}, {affiliateBroker5AccountSs58Id: {equalTo: $affiliateBrokerAccountSs58Id}}, {sourceAsset: {equalTo: $asset}}, {destAsset: {equalTo: $asset}}, {accountByMainBrokerAccountSs58Id: {alias: {includesInsensitive: $alias}}}], isInternal: {equalTo: false}}
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
                            swapRequestNativeId
                            sourceAsset
                            destAsset
                            baseAssetLeg1
                            baseAssetLeg2
                            ingressAmount
                            ingressValueUsd
                            egressAmount
                            egressValueUsd
                            inputAmount
                            inputValueUsd
                            intermediateAmount
                            intermediateValueUsd
                            outputAmount
                            outputValueUsd
                            refundAmount
                            refundValueUsd
                            networkFeeValueUsd
                            totalChunks
                            executedChunks
                            isDca
                            isBoosted
                            isOnChain
                            isInternal
                            isCcm
                            isVaultSwap
                            completedBlockId
                            completedBlockTimestamp
                            completedBlockDate
                            mainBrokerAccountSs58Id
                            mainBrokerFeeValueUsd
                            affiliateBroker1AccountSs58Id
                            affiliateBroker1FeeValueUsd
                            completedInSeconds
                            startedBlockDate
                            startedBlockId
                            startedBlockTimestamp
                            destinationAddress
                            outputAndIntermediateValueUsd
                            refundAddress
                            status
                            isInProgress
                            broker: accountByMainBrokerAccountSs58Id {
                                alias
                            }
                        }
                    }
                    totalCount
                }
            }
        "#;

        let variables = json!({
            "first": first.unwrap_or(30),
            "offset": offset.unwrap_or(0),
            "destinationOrRefundAddress": destination_address
        });

        Self::fetch_with_retry(base_url, &client, None, query, variables, "GetAllSwaps").await
    }
}
