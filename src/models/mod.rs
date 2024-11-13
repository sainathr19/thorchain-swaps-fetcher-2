use serde::{Deserialize, Serialize};
pub mod actions_model;

#[derive(Serialize, Deserialize, Debug)]
pub struct CurrentPrice {
    pub usd: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MarketData {
    pub current_price: CurrentPrice,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PriceFetchResponse {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub market_data: MarketData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CoinSearchData {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub api_symbol: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CoinSearchResponse {
    pub coins: Vec<CoinSearchData>,
}
