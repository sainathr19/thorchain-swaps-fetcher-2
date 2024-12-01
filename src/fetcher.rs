use std::collections::HashSet;
use std::sync::Arc;

use crate::db::PostgreSQL;
use crate::models::actions_model::SwapTransactionFromatted;
use crate::models::chainflip_swaps::ChainflipSwap;
use crate::models::closing_prices::ClosingPriceInterval;
use crate::utils::coingecko::CoinGecko;
use crate::utils::midgard::MidGard;
use crate::utils::transaction_handler::{TransactionError, TransactionHandler};
use crate::utils::{parse_f64, read_next_page_token_from_file, write_next_page_token_to_file};
use crate::SwapType;
use chrono::Utc;
use futures_util::lock::Mutex;

pub async fn fetch_chainflip_swaps(base_url: &str, pg: &PostgreSQL) -> Result<(), TransactionError> {
    let mut offset = 0;
    let limit = 20;

    'outer: loop {
        let resp = match crate::utils::chainflip::ChainFlip::fetch_chainflip_swaps(
            base_url,
            Some(limit),
            Some(offset),
            None,
        ).await {
            Ok(response) => response,
            Err(err) => {
                return Err(TransactionError::ApiError(format!(
                    "Error fetching Chainflip swaps: {:?}",
                    err
                )));
            }
        };

        let swaps = resp.data.allSwapRequests;

        // Process each swap
        for edge in swaps.edges {
            let node = &edge.node;
            
            // Get egress and channel data
            let egress = match &node.egressByEgressId {
                Some(e) => e,
                None => {
                    println!("Skipping swap {} - missing egress data", node.nativeId);
                    continue;
                }
            };

            let channel = match &node.swapChannelByDepositChannelId {
                Some(c) => c,
                None => {
                    println!("Skipping swap {} - missing channel data", node.nativeId);
                    continue;
                }
            };

            // Get date and time from timestamp
            let dt = chrono::DateTime::parse_from_rfc3339(&egress.eventByScheduledEventId.blockByBlockId.timestamp)
                .unwrap_or_else(|_| chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap());
            
            let date = dt.format("%Y-%m-%d").to_string();

            let formatted_data = ChainflipSwap {
                timestamp: dt.timestamp(),
                date,
                swap_id: node.nativeId.clone(),
                in_asset: node.sourceAsset.to_uppercase(),
                in_amount: if let Some(swaps) = &node.executedSwaps {
                    parse_f64(&swaps.aggregates.sum.swapInputAmount)
                        .unwrap_or_else(|e| {
                            println!("Error parsing input amount: {}", e);
                            0.0
                        })
                } else {
                    println!("No executed swaps data for {}", node.nativeId);
                    0.0
                },
                in_address: channel.depositAddress.clone(),
                in_amount_usd: if let Some(swaps) = &node.executedSwaps {
                    parse_f64(&swaps.aggregates.sum.swapInputValueUsd)
                        .unwrap_or_else(|e| {
                            println!("Error parsing input USD amount: {}", e);
                            0.0
                        })
                } else {
                    0.0
                },
                out_amount_usd: if let Some(swaps) = &node.executedSwaps {
                    parse_f64(&swaps.aggregates.sum.swapOutputValueUsd)
                        .unwrap_or_else(|e| {
                            println!("Error parsing output USD amount: {}", e);
                            0.0
                        })
                } else {
                    0.0
                },
                out_asset: node.destinationAsset.to_uppercase(),
                out_amount: if let Some(swaps) = &node.executedSwaps {
                    parse_f64(&swaps.aggregates.sum.swapOutputAmount)
                        .unwrap_or_else(|e| {
                            println!("Error parsing output amount: {}", e);
                            0.0
                        })
                } else {
                    0.0
                },
                out_address: node.destinationAddress.clone(),
            };

            // Insert into database with detailed error logging
            match pg.insert_chainflip_swap(formatted_data.clone()).await {
                Ok(_) => println!("Successfully inserted swap {:?} Date : {}", node.nativeId,formatted_data.date),
                Err(e) => {
                    println!("Error Message: {}", e);
                    break 'outer;
                }
            }
        }
        // Check if we have more pages
        if !swaps.pageInfo.hasNextPage {
            break 'outer;
        }

        offset += limit;
    }

    Ok(())
}

pub async fn fetch_btc_closing_price(pg:&PostgreSQL)-> Result<(),TransactionError>{
    let coingecko = match CoinGecko::init(){
        Ok(coingecko)=>{
            coingecko
        },
        Err(err)=>{
            println!("Error Initializing CoinGecko : {:?}",err);
            return Err(TransactionError::ApiError(String::from("Error Initializing CoinGecko")));
        }
    };

    let btc_coin_id = "bitcoin";
    let today = Utc::now();
    let coingecko_date = today.format("%d-%m-%Y").to_string();
    let current_date = today.format("%Y-%m-%d").to_string();

    let closing_price_usd = match coingecko.fetch_usd_price(&btc_coin_id, &coingecko_date).await{
        Ok(closing_price) => closing_price,
        Err(err)=>{
            println!("Error Fetching Closing Price : {:?}",err);
            return Err(TransactionError::PriceFetchError(btc_coin_id.to_string()));
        }
    };

    let closing_price_interval = ClosingPriceInterval{
        date : current_date.clone(),
        closing_price_usd
    };
    match pg.insert_closing_price(closing_price_interval).await{
        Ok(_)=>{
            println!("Closing Price Inserted Successfully for Date : {} Price : {}",&current_date,&closing_price_usd);
        }
        Err(err)=>{
            println!("Error Inserting Closing Price : {:?}",err);
        }
    }

    Ok(())
}
pub async fn _fetch_historical_data(base_url: &str,swap_type: SwapType) -> Result<(), TransactionError> {
    println!("Starting..");
    let pg = PostgreSQL::init().await.map_err(|e| {
        TransactionError::DatabaseError(format!("Error connecting to PostgreSQL: {:?}", e))
    })?;
    let transaction_handler = TransactionHandler;
    const TOKEN_FILE_PATH: &str = "next_page_token.txt";
    let mut next_page_token = read_next_page_token_from_file(TOKEN_FILE_PATH).unwrap_or_default();
    let mut transaction_batch : Vec<SwapTransactionFromatted> = Vec::new();
    let mut batch_count = 0;
    loop {
        let resp = match MidGard::fetch_actions_with_nextpage(base_url,     next_page_token.as_str()).await {
            Ok(resp) => resp,
            Err(err) => {
                println!("Error fetching actions data: {:?}. Retrying...", err);
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                continue;
            }
        };

        if resp.actions.is_empty() {
            println!("No more actions to process, exiting loop.");
            break;
        }

        let processed_transactions = transaction_handler.process_transactions(&resp.actions,swap_type.clone()).await;
        match processed_transactions{
            Ok(val)=>{
                transaction_batch.extend(val);
                batch_count+=1;
                println!("Processed Batch : {}",&batch_count);
                next_page_token = resp.meta.nextPageToken.clone();
                if let Err(e) = write_next_page_token_to_file(&next_page_token, TOKEN_FILE_PATH) {
                    return Err(TransactionError::FileError(format!(
                        "Error writing next page token to file: {:?}",
                        e
                    )));
                }
            }
            Err(err)=>{
                println!("Error parsing Transactions : {:?}",err);
                return Err(TransactionError::ProcessingError(format!(
                    "Error processing transaction: {:?}",
                    err
                )));
            }
        }

        if batch_count>=20{
            let table_name = match swap_type {
                SwapType::NATIVE => "native_swaps_thorchain",
                SwapType::TRADE => "swap_history_test"
            };
            let insertion_response = pg.insert_bulk(table_name, transaction_batch.clone()).await;
            match insertion_response {
                Ok(_)=>{
                    println!("Batch insertion Successfull of : {}",&transaction_batch.len());
                }
                Err(err)=>{
                    println!("Error inserting Batch : {:?}",err);
                }
                
            }
            batch_count=0;
            transaction_batch.clear();
            println!("Batch Cleared. Size After Clear: {}", &transaction_batch.len());
        }

        // let process_response =
        //     transaction_handler.process_and_insert_transaction(&pg, &resp.actions).await;

        // match process_response {
        //     Ok(_) => {
        //         next_page_token = resp.meta.nextPageToken.clone();
        //         if let Err(e) = write_next_page_token_to_file(&next_page_token,TOKEN_FILE_PATH) {
        //             return Err(TransactionError::FileError(format!(
        //                 "Error writing next page token to file: {:?}",
        //                 e
        //             )));
        //         }
        //         println!("Updated next page token: {}", &next_page_token);
        //     }
        //     Err(err) => {
        //         return Err(TransactionError::ProcessingError(format!(
        //             "Error processing transaction: {:?}",
        //             err
        //         )));
        //     }
        // }
    }

    Ok(())
}
pub async fn fetch_latest_data(pg: &PostgreSQL,base_url: &str,swap_type: SwapType) -> Result<(), TransactionError> {
    let transaction_handler = TransactionHandler;
    let table_name = match swap_type {
        SwapType::NATIVE => "native_swaps_thorchain",
        SwapType::TRADE => "swap_history_test"
    };
    let latest_timestamp = match pg.fetch_latest_timestamp(table_name).await {
        Ok(Some(timestamp)) => timestamp,
        Ok(None) => Utc::now().timestamp() as i64,
        Err(err) => {
            return Err(TransactionError::DatabaseError(format!(
                "Error fetching the latest timestamp: {:?}",
                err
            )));
        }
    };

    let pg_clone = pg.clone();
    let latest_timestamp_str = latest_timestamp.to_string();

    // Fetch actions with the latest timestamp
    let mut resp = match MidGard::fetch_actions_with_timestamp(base_url, &latest_timestamp_str).await {
        Ok(response) => response,
        Err(err) => {
            return Err(TransactionError::ApiError(format!(
                "Error fetching actions with timestamp: {:?}",
                err
            )));
        }
    };
    let mut actions = resp.actions.clone();
    actions.reverse();
    let process_response =
        transaction_handler.process_and_insert_transaction(&pg_clone, &actions,swap_type.clone()).await;
    match process_response {
        Ok(_) => (),
        Err(err) => {
            return Err(TransactionError::ProcessingError(format!(
                "Error processing transaction: {:?}",
                err
            )));
        }
    };

    while !resp.actions.is_empty() {
        let prev_page_token = resp.meta.prevPageToken.clone();
        resp = match MidGard::fetch_actions_with_prevpage(base_url,prev_page_token.as_str()).await {
            Ok(response) => response,
            Err(err) => {
                return Err(TransactionError::ApiError(format!(
                    "Error fetching previous page actions: {:?}",
                    err
                )));
            }
        };

        let process_response =
            transaction_handler.process_and_insert_transaction(&pg_clone, &resp.actions,swap_type.clone()).await;
        match process_response {
            Ok(_) => (),
            Err(err) => {
                return Err(TransactionError::ProcessingError(format!(
                    "Error processing transaction: {:?}",
                    err
                )));
            }
        };
    }

    println!("Latest Data Updated at : {}", latest_timestamp_str);
    Ok(())
}
pub async fn retry_pending_transactions(pg: &PostgreSQL,base_url: &str,pending_ids:Arc<Mutex<HashSet<String>>>,swap_type: SwapType) -> Result<(), TransactionError> {
    let transaction_handler = TransactionHandler;
    let pending_txn_ids = pending_ids.lock().await.clone();
    println!("Fetching Pending Transactions.. : {:?}",&pending_txn_ids);
    
    for transaction_id in pending_txn_ids {
        let resp = match MidGard::fetch_action_with_transactionid(base_url,transaction_id).await {
            Ok(response) => response,
            Err(err) => {
                return Err(TransactionError::ApiError(format!(
                    "Error fetching previous page actions: {:?}",
                    err
                )));
            }
        };
        let pg_clone = pg.clone();
        let process_response =
            transaction_handler.process_and_insert_transaction(&pg_clone, &resp.actions,swap_type.clone()).await;
        match process_response {
            Ok(_) => (),
            Err(err) => {
                return Err(TransactionError::ProcessingError(format!(
                    "Error processing transaction: {:?}",
                    err
                )));
            }
        };
    }
    Ok(())
}