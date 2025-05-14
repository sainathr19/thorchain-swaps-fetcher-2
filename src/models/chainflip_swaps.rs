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
    pub swapRequestNativeId: String,
    pub sourceAsset: String,
    pub destAsset: String,
    pub baseAssetLeg1: Option<String>,
    pub baseAssetLeg2: Option<String>,
    pub ingressAmount: Option<String>,
    pub ingressValueUsd: Option<String>,
    pub egressAmount: Option<String>,
    pub egressValueUsd: Option<String>,
    pub inputAmount: Option<String>,
    pub inputValueUsd: Option<String>,
    pub intermediateAmount: Option<String>,
    pub intermediateValueUsd: Option<String>,
    pub outputAmount: Option<String>,
    pub outputValueUsd: Option<String>,
    pub refundAmount: Option<String>,
    pub refundValueUsd: Option<String>,
    pub networkFeeValueUsd: Option<String>,
    pub totalChunks: Option<i32>,
    pub executedChunks: Option<i32>,
    pub isDca: Option<bool>,
    pub isBoosted: Option<bool>,
    pub isOnChain: bool,
    pub isInternal: bool,
    pub isCcm: Option<bool>,
    pub isVaultSwap: Option<bool>,
    pub completedBlockId: Option<i64>,
    pub completedBlockTimestamp: Option<String>,
    pub completedBlockDate: Option<String>,
    pub mainBrokerAccountSs58Id: Option<String>,
    pub mainBrokerFeeValueUsd: Option<String>,
    pub startedBlockDate: Option<String>,
    pub startedBlockId: Option<i64>,
    pub startedBlockTimestamp: Option<String>,
    pub destinationAddress: String,
    pub outputAndIntermediateValueUsd: Option<String>,
    pub refundAddress: Option<String>,
    pub status: String,
    pub isInProgress: bool,
    pub broker: Option<Broker>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Broker {
    pub alias: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChainflipSwap {
    pub timestamp: i64,
    pub date: String,
    pub swap_id: String,
    pub in_asset: String,
    pub in_amount: f64,
    pub in_amount_usd: f64,
    pub in_address: Option<String>,
    pub out_asset: String,
    pub out_amount: f64,
    pub out_amount_usd: f64,
    pub out_address: String,
    pub broker: Option<String>,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChainflipSwapDetailed {
    pub timestamp: i64,
    pub date: String,
    pub swap_id: String,
    pub source_asset: String,
    pub dest_asset: String,
    pub base_asset_leg1: Option<String>,
    pub base_asset_leg2: Option<String>,
    pub ingress_amount: f64,
    pub ingress_value_usd: f64,
    pub input_amount: f64,
    pub input_value_usd: f64,
    pub output_amount: f64,
    pub output_value_usd: f64,
    pub started_block_date: Option<String>,
    pub started_block_id: Option<i64>,
    pub started_block_timestamp: Option<String>,
    pub destination_address: String,
    pub refund_address: Option<String>,
    pub status: String,
    pub broker: Option<String>,
}
