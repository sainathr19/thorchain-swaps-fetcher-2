// use super::{calculate_transaction_amount, coingecko::COINGECKO_INSTANCE};
use crate::{
    db::PostgreSQL,
    models::actions_model::{SwapTransaction, SwapTransactionFromatted, TransactionData},
    utils::{
        asset_name_from_pool, convert_nano_to_sec, convert_to_standard_unit,
        format_epoch_timestamp, parse_f64,
    }, SwapType,
};
use reqwest::Error as ReqwestError;
use sqlx::Error as SqlxError;
use std::fmt;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use lazy_static::lazy_static;

use super::asset_name_from_trade_pool;

lazy_static! {
    pub static ref TRADE_SWAPS_PENDING_IDS: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
}
lazy_static! {
    pub static ref NATIVE_SWAPS_PENDING_IDS: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
}

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
    ) -> Result<(String, f64, String), TransactionError> {
        let coin = info.coins.get(0).ok_or(TransactionError::MissingInCoin)?;

        let token_amount = parse_f64(&coin.amount).expect("Floating point parse error");
        let standard_amount = convert_to_standard_unit(token_amount, 8);

        let asset_name =
            asset_name_from_trade_pool(&coin.asset).ok_or(TransactionError::MissingAssetName)?;
        let address = info.address.clone();

        Ok((asset_name, standard_amount, address))
    }

    pub async fn parse_transaction(
        &self,
        swap: &SwapTransaction,
    ) -> Result<SwapTransactionFromatted, TransactionError> {
        let (swap_date, swap_time) = format_epoch_timestamp(&swap.date).expect("Formatting error");
        let epoc_timestamp = parse_f64(convert_nano_to_sec(&swap.date).as_str()).unwrap() as i64;   
        let tx_id = swap
            .in_data
            .get(0)
            .and_then(|data| data.txID.clone())
            .ok_or(TransactionError::MissingTxId)?;
        println!("Current Progress Date : {}",&swap_date);
        let handler = TransactionHandler;
        let in_data = swap.in_data.get(0).ok_or(TransactionError::MissingInData)?;
        let (in_asset, in_amount, in_address) = handler.parse_data(in_data).await?;
    
        let mut out_data = swap.out_data.clone();
        out_data.reverse();
    
        let out_data_1 = out_data.get(0).ok_or(TransactionError::MissingOutData)?;
        let (asset_1, amount_1, address_1) = handler.parse_data(out_data_1).await?;
    
        let (out_asset_1, out_amount_1, out_address_1, out_asset_2, out_amount_2, out_address_2) = 
            if let Some(out_data_2) = out_data.get(1) {
                let (asset_2, amount_2, address_2) = handler.parse_data(out_data_2).await?;
                
                if asset_1 == "THOR.RUNE" {
                    (asset_2, amount_2, address_2, Some(asset_1), Some(amount_1), Some(address_1))
                } else if asset_2 == "THOR.RUNE" {
                    (asset_1, amount_1, address_1, Some(asset_2), Some(amount_2), Some(address_2))
                } else {
                    (asset_1, amount_1, address_1, Some(asset_2), Some(amount_2), Some(address_2))
                }
            } else {
                (asset_1, amount_1, address_1, None, None, None)
            };
    
        Ok(SwapTransactionFromatted {
            timestamp: epoc_timestamp,
            date: swap_date,
            time: swap_time,
            in_asset,
            in_amount,
            out_asset_1,
            out_amount_1,
            in_address,
            out_address_1,
            tx_id,
            out_asset_2,
            out_amount_2,
            out_address_2,
            status : swap.status.clone()
        })
    }

    pub async fn process_and_insert_transaction(
        &self,
        pg: &PostgreSQL,
        actions: &Vec<SwapTransaction>,
        swap_type: SwapType
    ) -> Result<(), TransactionError> {
        let processed_transactions = self.process_transactions(actions,swap_type.clone()).await.unwrap();
        let table_name = match swap_type {
            SwapType::NATIVE => "btc_user_data",
            SwapType::TRADE => "swap_history_test"
        };
        if let Err(err) = pg.insert_bulk(table_name, processed_transactions).await{
            println!("Error Inserting Bulk : {:?}",err);
        }
        // for swap in processed_transactions{
        //     if let Err(err) = pg.insert_new_record(swap.clone()).await {
        //         println!("Error during insertion: {:?}", err);
        //     }
        // }
        Ok(())
    }

    pub async fn process_transactions(
        &self,
        actions: &Vec<SwapTransaction>,
        swap_type: SwapType
    ) -> Result<Vec<SwapTransactionFromatted>, TransactionError> {
        let mut result : Vec<SwapTransactionFromatted> = Vec::new();
        let mut pending_count = 0;
        for swap in actions {
            let transaction_info = self.parse_transaction(&swap).await;
            let transaction_info = match transaction_info {
                Ok(val) => val,
                Err(err) => {
                    println!("Error parsing transaction: {:?}", err);
                    continue;
                }
            };
            if transaction_info.status!="success"{
                self.track_pending_transaction(transaction_info.tx_id,swap_type.clone()).await;
                pending_count+=1;
            }else{
                result.push(transaction_info);
            }
        }
        println!("Pending Transactions in batch : {}",&pending_count);
        Ok(result)
    }
    
    pub async fn track_pending_transaction(&self,transaction_id: String,swap_type: SwapType) {
        let mut pending_txn_ids = match swap_type {
            SwapType::NATIVE => NATIVE_SWAPS_PENDING_IDS.lock().await,
            SwapType::TRADE => TRADE_SWAPS_PENDING_IDS.lock().await,
        };
        pending_txn_ids.insert(transaction_id);
    }
}
