use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapResponse {
    pub data: SwapData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapData {
    pub allSwapRequests: SwapRequests,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapRequests {
    pub pageInfo: PageInfo,
    pub edges: Vec<Edge>,
    pub totalCount: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PageInfo {
    pub hasPreviousPage: bool,
    pub startCursor: String,
    pub hasNextPage: bool,
    pub endCursor: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Edge {
    pub node: SwapNode,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapNode {
    pub id: i64,
    pub nativeId: String,
    pub sourceAsset: String,
    pub depositAmount: String,
    pub depositValueUsd: String,
    pub destinationAsset: String,
    pub destinationAddress: String,
    pub r#type: String,
    #[serde(default)]
    pub egressId: Option<i64>,
    pub egressByEgressId: Option<Egress>,
    pub executedSwaps: Option<ExecutedSwaps>,
    pub swapChannelByDepositChannelId: Option<SwapChannel>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutedSwaps {
    pub totalCount: i64,
    pub aggregates: Aggregates,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Aggregates {
    pub sum: SwapSums,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapSums {
    pub swapInputAmount: String,
    pub swapInputValueUsd: String,
    pub intermediateAmount: String,
    pub intermediateValueUsd: String,
    pub swapOutputAmount: String,
    pub swapOutputValueUsd: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Egress {
    pub amount: String,
    pub valueUsd: String,
    pub eventByScheduledEventId: Event,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    pub blockByBlockId: Block,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Block {
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapChannel {
    pub sourceAsset: String,
    pub depositAddress: String,
    pub destinationAsset: String,
    pub destinationAddress: String,
}

#[derive(Debug, Serialize, Deserialize,Clone)]
pub struct ChainflipSwap {
    pub timestamp: i64,
    pub date: String,
    pub swap_id: String,
    pub in_asset: String,
    pub in_amount: f64,
    pub in_amount_usd: f64,
    pub in_address: String,
    pub out_asset: String,
    pub out_amount: f64,
    pub out_amount_usd: f64,
    pub out_address: String,
}   
