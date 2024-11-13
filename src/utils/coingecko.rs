use crate::models::{CoinSearchResponse, PriceFetchResponse};
use dotenv::dotenv;
use once_cell::sync::Lazy;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, Error as ReqwestError,
};
use std::sync::Arc;
use std::{collections::HashMap, env};
use tokio::sync::RwLock;

pub struct CoinGecko {
    client: Client,
    base_url: String,
    coin_id: HashMap<String, String>,
}

impl CoinGecko {
    pub fn init() -> Result<Self, ReqwestError> {
        dotenv().ok();

        let coingecko_base_url = env::var("COINGECKO_BASE_URL").expect("Coingecko BASE URL needed");
        let coingecko_api_key = env::var("COINGECKO_API_KEY").expect("Coingecko API KEY needed");

        let mut headers = HeaderMap::new();
        headers.insert(
            "x-cg-demo-api-key",
            HeaderValue::from_str(&coingecko_api_key).expect("Invalid Header"),
        );
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        let client = Client::builder().default_headers(headers).build()?;
        let coin_id = HashMap::new();

        Ok(Self {
            client,
            base_url: coingecko_base_url,
            coin_id,
        })
    }

    // Fetch the USD price for a specific coin and date
    pub async fn fetch_usd_price(&self, coin_id: &str, date: &str) -> Result<f64, ReqwestError> {
        let url = format!("{}/coins/{}/history?date={}", self.base_url, coin_id, date);

        let response = self.client.get(&url).send().await?.error_for_status()?;
        let resp: PriceFetchResponse = response.json().await?;

        Ok(resp.market_data.current_price.usd)
    }

    // Search for a coin by name
    pub async fn search_coin(&self, coin_name: &str) -> Result<Option<String>, ReqwestError> {
        let url = format!("{}/search?query={}", self.base_url, coin_name);

        let response = self.client.get(&url).send().await?.error_for_status()?;

        let resp: CoinSearchResponse = response.json().await?;

        Ok(match resp.coins.get(0) {
            Some(coin) => Some(coin.id.clone()),
            None => None,
        })
    }

    pub fn get_coin_id(&self, asset_name: &str) -> Option<String> {
        self.coin_id.get(asset_name).cloned()
    }

    pub fn add_coin_id(&mut self, coin_name: &str, coin_id: &str) {
        self.coin_id
            .insert(coin_name.to_string(), coin_id.to_string());
    }
}

pub static COINGECKO_INSTANCE: Lazy<Arc<RwLock<CoinGecko>>> = Lazy::new(|| {
    Arc::new(RwLock::new(
        CoinGecko::init().expect("Failed to initialize CoinGecko client"),
    ))
});
