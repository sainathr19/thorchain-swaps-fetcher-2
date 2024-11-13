use super::{calculate_transaction_amount, coingecko::COINGECKO_INSTANCE};
use crate::{
    db::MySQL,
    models::actions_model::{SwapTransaction, SwapTransactionFromatted, TransactionData},
    utils::{
        asset_name_from_pool, coin_name_from_pool, convert_nano_to_sec, convert_to_standard_unit,
        format_epoch_timestamp, parse_f64,
    },
};
use reqwest::Error as ReqwestError;
use sqlx::Error as SqlxError;
use std::fmt;

#[derive(Debug)]
pub enum TransactionError {
    MissingInCoin,
    MissingAssetName,
    CoinNotFound(String),
    PriceFetchError(String),
    MissingTxId,
    MissingInData,
    MissingOutData,
    SqlxError(SqlxError),
    ApiError(String),
    FileError(String),
    ProcessingError(String),
    DatabaseError(String),
}

impl fmt::Display for TransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionError::MissingInCoin => write!(f, "Missing in_coin"),
            TransactionError::MissingAssetName => write!(f, "Error parsing asset name"),
            TransactionError::CoinNotFound(coin_name) => write!(f, "Coin not found: {}", coin_name),
            TransactionError::PriceFetchError(coin_name) => {
                write!(f, "Price fetch failed for: {}", coin_name)
            }
            TransactionError::MissingTxId => write!(f, "Missing or invalid TxId"),
            TransactionError::MissingInData => write!(f, "No In Data Found"),
            TransactionError::MissingOutData => write!(f, "No Out Data Found"),
            TransactionError::SqlxError(err) => write!(f, "SQLx error: {}", err),
            TransactionError::ApiError(err) => write!(f, "API error: {}", err),
            TransactionError::FileError(err) => write!(f, "File operation error: {}", err),
            TransactionError::ProcessingError(err) => write!(f, "Processing error: {}", err),
            TransactionError::DatabaseError(err) => write!(f, "Database connection error: {}", err),
        }
    }
}

impl From<ReqwestError> for TransactionError {
    fn from(err: ReqwestError) -> Self {
        TransactionError::PriceFetchError(err.to_string())
    }
}

impl From<SqlxError> for TransactionError {
    fn from(err: SqlxError) -> Self {
        TransactionError::SqlxError(err)
    }
}

pub struct TransactionHandler;

impl TransactionHandler {
    pub async fn parse_data(
        &self,
        info: &TransactionData,
        swap_date: &str,
    ) -> Result<(String, f64, f64, String), TransactionError> {
        let in_coin = info.coins.get(0).ok_or(TransactionError::MissingInCoin)?;
        let coin_name =
            coin_name_from_pool(&in_coin.asset).ok_or(TransactionError::MissingAssetName)?;

        let in_amount = parse_f64(&in_coin.amount).expect("Floating point parse error");

        let in_amount_usd = self
            .convert_amount_to_usd(&coin_name, swap_date, in_amount)
            .await?;

        let in_asset =
            asset_name_from_pool(&in_coin.asset).ok_or(TransactionError::MissingAssetName)?;
        let in_address = info.address.clone();

        Ok((in_asset, in_amount, in_amount_usd, in_address))
    }
    pub async fn convert_amount_to_usd(
        &self,
        asset_name: &str,
        date: &str,
        amount: f64,
    ) -> Result<f64, TransactionError> {
        let mut coingecko = COINGECKO_INSTANCE.write().await;

        let coin_id = match coingecko.get_coin_id(&asset_name) {
            Some(coin_id) => coin_id,
            None => {
                let coin_id = coingecko.search_coin(asset_name).await.map_err(|_| {
                    println!("Error searching for coin ID for asset: {}", asset_name);
                    TransactionError::CoinNotFound(asset_name.to_string())
                })?;
                let coin_id  =coin_id.ok_or_else(|| {
                    println!("Coin ID not found after search for asset: {}", asset_name);
                    TransactionError::CoinNotFound(asset_name.to_string())
                })?;
                coingecko.add_coin_id(&asset_name, &coin_id);
                coin_id
            }
        };
        let price_on_date = coingecko
            .fetch_usd_price(coin_id.as_str(), date)
            .await
            .map_err(|_| {
                println!(
                    "Error fetching price for coin ID: {} on date: {}",
                    coin_id, date
                );
                TransactionError::PriceFetchError(coin_id.clone())
            })?;
        let amount = convert_to_standard_unit(amount, 8);
        let t_amount = calculate_transaction_amount(amount, price_on_date);

        Ok((t_amount * 100.0).round() / 100.0)
    }

    pub async fn parse_transaction(
        swap: &SwapTransaction,
    ) -> Result<SwapTransactionFromatted, TransactionError> {
        // Parse swap_date & swap_time
        let (swap_date, swap_time) = format_epoch_timestamp(&swap.date).expect("Formatting error");
        let epoc_timestamp = parse_f64(convert_nano_to_sec(&swap.date).as_str()).unwrap() as i64;

        println!("Current Progress Date : {}", &swap_date);

        // Parse tx_id from in_data
        let tx_id = swap
            .in_data
            .get(0)
            .and_then(|data| data.txID.clone())
            .ok_or(TransactionError::MissingTxId)?;

        let handler = TransactionHandler;

        // Parse In Data
        let in_data = swap.in_data.get(0).ok_or(TransactionError::MissingInData)?;
        let (in_asset, in_amount, in_amount_usd, in_address) =
            handler.parse_data(in_data, &swap_date).await?;

        let mut out_data = swap.out_data.clone();
        out_data.reverse();

        // Parse Out Data
        let out_data_1 = out_data.get(0).ok_or(TransactionError::MissingOutData)?;
        let (out_asset_1, out_amount_1, out_amount_1_usd, out_address_1) =
            handler.parse_data(out_data_1, &swap_date).await?;

        let (out_asset_2, out_amount_2, out_amount_2_usd, out_address_2) = if let Some(val) =
            out_data.get(1)
        {
            let (asset, amount, amount_usd, address) = handler.parse_data(val, &swap_date).await?;
            (Some(asset), Some(amount), Some(amount_usd), Some(address))
        } else {
            (None, None, None, None)
        };

        Ok(SwapTransactionFromatted {
            timestamp: epoc_timestamp,
            date: swap_date,
            time: swap_time,
            in_asset,
            in_amount,
            in_amount_usd,
            out_asset_1,
            out_amount_1,
            out_amount_1_usd,
            in_address,
            out_address_1,
            tx_id,
            out_asset_2,
            out_amount_2,
            out_amount_2_usd,
            out_address_2,
        })
    }

    pub async fn process_and_insert_transaction(
        mysql: &MySQL,
        actions: &Vec<SwapTransaction>,
    ) -> Result<(), TransactionError> {
        
        for swap in actions {
            if swap.status != "success" {
                println!("Transaction Pending");
                continue;
            }
            let transaction_info = TransactionHandler::parse_transaction(&swap).await;

            let transaction_info = match transaction_info {
                Ok(val) => val,
                Err(err) => {
                    println!("Error parsing transaction: {:?}", err);
                    continue;
                }
            };
            if let Err(err) = mysql.insert_new_record(transaction_info.clone()).await {
                println!("Error during insertion: {:?}", err);
            } else {
                println!("Insertion Successful for Id : {}", &transaction_info.tx_id);
            }
        }

        Ok(())
    }
}
