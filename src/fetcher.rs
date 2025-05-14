use std::collections::HashSet;
use std::sync::Arc;

use crate::db::PostgreSQL;
use crate::models::actions_model::SwapTransactionFromatted;
use crate::models::chainflip_swaps::{ChainflipSwap, ChainflipSwapDetailed};
use crate::models::closing_prices::ClosingPriceInterval;
use crate::utils::coingecko::CoinGecko;
use crate::utils::midgard::MidGard;
use crate::utils::transaction_handler::{TransactionError, TransactionHandler};
use crate::utils::{parse_f64, read_next_page_token_from_file, write_next_page_token_to_file};
use crate::SwapType;
use chrono::Utc;
use futures_util::lock::Mutex;

pub async fn fetch_btc_closing_price(pg: &PostgreSQL) -> Result<(), TransactionError> {
    let coingecko = match CoinGecko::init() {
        Ok(coingecko) => coingecko,
        Err(err) => {
            println!("Error Initializing CoinGecko : {:?}", err);
            return Err(TransactionError::ApiError(String::from(
                "Error Initializing CoinGecko",
            )));
        }
    };

    let btc_coin_id = "bitcoin";
    let today = Utc::now();
    let coingecko_date = today.format("%d-%m-%Y").to_string();
    let current_date = today.format("%Y-%m-%d").to_string();

    let closing_price_usd = match coingecko
        .fetch_usd_price(&btc_coin_id, &coingecko_date)
        .await
    {
        Ok(closing_price) => closing_price,
        Err(err) => {
            println!("Error Fetching Closing Price : {:?}", err);
            return Err(TransactionError::PriceFetchError(btc_coin_id.to_string()));
        }
    };

    let closing_price_interval = ClosingPriceInterval {
        date: current_date.clone(),
        closing_price_usd,
    };
    match pg.insert_closing_price(closing_price_interval).await {
        Ok(_) => {
            println!(
                "Closing Price Inserted Successfully for Date : {} Price : {}",
                &current_date, &closing_price_usd
            );
        }
        Err(err) => {
            println!("Error Inserting Closing Price : {:?}", err);
        }
    }

    Ok(())
}
pub async fn _fetch_historical_data(
    base_url: &str,
    swap_type: SwapType,
) -> Result<(), TransactionError> {
    println!("Starting..");
    let pg = PostgreSQL::init().await.map_err(|e| {
        TransactionError::DatabaseError(format!("Error connecting to PostgreSQL: {:?}", e))
    })?;
    let transaction_handler = TransactionHandler;
    const TOKEN_FILE_PATH: &str = "next_page_token.txt";
    let mut next_page_token = read_next_page_token_from_file(TOKEN_FILE_PATH).unwrap_or_default();
    let mut transaction_batch: Vec<SwapTransactionFromatted> = Vec::new();
    let mut batch_count = 0;
    loop {
        let resp =
            match MidGard::fetch_actions_with_nextpage(base_url, next_page_token.as_str()).await {
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

        let processed_transactions = transaction_handler
            .process_transactions(&resp.actions, swap_type.clone())
            .await;
        match processed_transactions {
            Ok(val) => {
                transaction_batch.extend(val);
                batch_count += 1;
                println!("Processed Batch : {}", &batch_count);
                next_page_token = resp.meta.nextPageToken.clone();
                if let Err(e) = write_next_page_token_to_file(&next_page_token, TOKEN_FILE_PATH) {
                    return Err(TransactionError::FileError(format!(
                        "Error writing next page token to file: {:?}",
                        e
                    )));
                }
            }
            Err(err) => {
                println!("Error parsing Transactions : {:?}", err);
                return Err(TransactionError::ProcessingError(format!(
                    "Error processing transaction: {:?}",
                    err
                )));
            }
        }

        if batch_count >= 20 {
            let table_name = match swap_type {
                SwapType::NATIVE => "native_swaps_thorchain",
                SwapType::TRADE => "swap_history_test",
            };
            let insertion_response = pg.insert_bulk(table_name, transaction_batch.clone()).await;
            match insertion_response {
                Ok(_) => {
                    println!(
                        "Batch insertion Successfull of : {}",
                        &transaction_batch.len()
                    );
                }
                Err(err) => {
                    println!("Error inserting Batch : {:?}", err);
                }
            }
            batch_count = 0;
            transaction_batch.clear();
            println!(
                "Batch Cleared. Size After Clear: {}",
                &transaction_batch.len()
            );
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
pub async fn fetch_latest_data(
    pg: &PostgreSQL,
    base_url: &str,
    swap_type: SwapType,
) -> Result<(), TransactionError> {
    let transaction_handler = TransactionHandler;
    let table_name = match swap_type {
        SwapType::NATIVE => "native_swaps_thorchain",
        SwapType::TRADE => "swap_history_test",
    };

    // These tables use i64 (INT8) for timestamps
    let latest_timestamp = match pg.fetch_latest_timestamp_i64(table_name).await {
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
    let mut resp =
        match MidGard::fetch_actions_with_timestamp(base_url, &latest_timestamp_str).await {
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
    let process_response = transaction_handler
        .process_and_insert_transaction(&pg_clone, &actions, swap_type.clone())
        .await;
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
        resp = match MidGard::fetch_actions_with_prevpage(base_url, prev_page_token.as_str()).await
        {
            Ok(response) => response,
            Err(err) => {
                return Err(TransactionError::ApiError(format!(
                    "Error fetching previous page actions: {:?}",
                    err
                )));
            }
        };

        let process_response = transaction_handler
            .process_and_insert_transaction(&pg_clone, &resp.actions, swap_type.clone())
            .await;
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
pub async fn retry_pending_transactions(
    pg: &PostgreSQL,
    base_url: &str,
    pending_ids: Arc<Mutex<HashSet<String>>>,
    swap_type: SwapType,
) -> Result<(), TransactionError> {
    let transaction_handler = TransactionHandler;
    let pending_txn_ids = pending_ids.lock().await.clone();
    println!("Fetching Pending Transactions.. : {:?}", &pending_txn_ids);

    for transaction_id in pending_txn_ids {
        let resp = match MidGard::fetch_action_with_transactionid(base_url, transaction_id).await {
            Ok(response) => response,
            Err(err) => {
                return Err(TransactionError::ApiError(format!(
                    "Error fetching previous page actions: {:?}",
                    err
                )));
            }
        };
        let pg_clone = pg.clone();
        let process_response = transaction_handler
            .process_and_insert_transaction(&pg_clone, &resp.actions, swap_type.clone())
            .await;
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
pub async fn fetch_daily_data(
    pg: &PostgreSQL,
    base_url: &str,
    swap_type: SwapType,
    day_start_timestamp: i64,
) -> Result<(), TransactionError> {
    let transaction_handler = TransactionHandler;
    let pg_clone = pg.clone();
    let start_timestamp = day_start_timestamp.to_string();

    let mut resp = match MidGard::fetch_actions_with_timestamp(base_url, &start_timestamp).await {
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
    let process_response = transaction_handler
        .process_and_insert_transaction(&pg_clone, &actions, swap_type.clone())
        .await;
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
        resp = match MidGard::fetch_actions_with_prevpage(base_url, prev_page_token.as_str()).await
        {
            Ok(response) => response,
            Err(err) => {
                return Err(TransactionError::ApiError(format!(
                    "Error fetching previous page actions: {:?}",
                    err
                )));
            }
        };

        let process_response = transaction_handler
            .process_and_insert_transaction(&pg_clone, &resp.actions, swap_type.clone())
            .await;
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

pub async fn fetch_chainflip_swaps_incremental(
    base_url: &str,
    pg: &PostgreSQL,
) -> Result<(), TransactionError> {
    println!("Starting incremental Chainflip swaps fetch");

    // Get the latest timestamp from the database
    let latest_timestamp = match pg.fetch_latest_timestamp("chainflip_swaps_detailed").await {
        Ok(Some(timestamp)) => timestamp as i64, // Cast from i32 to i64
        Ok(None) => {
            println!("No existing records found, starting from scratch");
            0 // Start from the beginning if no records exist
        }
        Err(err) => {
            return Err(TransactionError::DatabaseError(format!(
                "Error fetching the latest timestamp: {:?}",
                err
            )));
        }
    };

    println!("Latest timestamp in database: {}", latest_timestamp);

    let mut offset = 0;
    let limit = 30;
    let mut total_fetched = 0;
    let mut total_inserted = 0;
    let mut total_skipped = 0;
    let mut found_existing_records = false;

    'outer: loop {
        println!("Fetching batch: offset={}, limit={}", offset, limit);
        let resp = match crate::utils::chainflip::ChainFlip::fetch_chainflip_swaps(
            base_url,
            Some(limit),
            Some(offset),
            None,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => {
                println!("API Error: {:?}", err);
                return Err(TransactionError::ApiError(format!(
                    "Error fetching Chainflip swaps: {:?}",
                    err
                )));
            }
        };

        let swaps = resp.data.allSwapRequests;
        println!("Retrieved {} swaps", swaps.edges.len());

        if swaps.edges.is_empty() {
            println!("No more swaps to fetch, ending loop");
            break 'outer;
        }

        // Process each swap
        for edge in swaps.edges {
            let node = &edge.node;

            // Only process completed successful swaps
            if node.status != "SUCCESS" {
                println!(
                    "Skipping swap {} - status: {}",
                    node.swapRequestNativeId, node.status
                );
                continue;
            }

            // Parse timestamp from completedBlockTimestamp or startedBlockTimestamp
            let timestamp_string = match (
                node.completedBlockTimestamp.as_ref(),
                node.startedBlockTimestamp.as_ref(),
            ) {
                (Some(completed), _) => completed.clone(),
                (_, Some(started)) => started.clone(),
                _ => "1970-01-01T00:00:00Z".to_string(),
            };

            let dt = chrono::DateTime::parse_from_rfc3339(&timestamp_string).unwrap_or_else(|_| {
                chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap()
            });

            let record_timestamp = dt.timestamp();

            // Skip records that are older than or equal to our latest timestamp
            if record_timestamp <= latest_timestamp {
                println!(
                    "Found existing record: id={}, timestamp={} (latest={}), skipping this and all older records",
                    node.swapRequestNativeId, record_timestamp, latest_timestamp
                );
                found_existing_records = true;
                break;
            }

            let date = dt.format("%Y-%m-%d").to_string();

            // Determine broker name
            let broker_name = match &node.broker {
                Some(broker) => broker.alias.clone(),
                None => None,
            };

            let formatted_data = ChainflipSwapDetailed {
                timestamp: record_timestamp,
                date,
                swap_id: node.swapRequestNativeId.clone(),
                source_asset: node.sourceAsset.to_uppercase(),
                dest_asset: node.destAsset.to_uppercase(),
                base_asset_leg1: node.baseAssetLeg1.clone().map(|a| a.to_uppercase()),
                base_asset_leg2: node.baseAssetLeg2.clone().map(|a| a.to_uppercase()),
                ingress_amount: node
                    .ingressAmount
                    .as_ref()
                    .and_then(|amount| parse_f64(amount).ok())
                    .unwrap_or(0.0),
                ingress_value_usd: node
                    .ingressValueUsd
                    .as_ref()
                    .and_then(|amount| parse_f64(amount).ok())
                    .unwrap_or(0.0),
                input_amount: node
                    .inputAmount
                    .as_ref()
                    .and_then(|amount| parse_f64(amount).ok())
                    .unwrap_or(0.0),
                input_value_usd: node
                    .inputValueUsd
                    .as_ref()
                    .and_then(|amount| parse_f64(amount).ok())
                    .unwrap_or(0.0),
                output_amount: node
                    .outputAmount
                    .as_ref()
                    .and_then(|amount| parse_f64(amount).ok())
                    .unwrap_or(0.0),
                output_value_usd: node
                    .outputValueUsd
                    .as_ref()
                    .and_then(|amount| parse_f64(amount).ok())
                    .unwrap_or(0.0),
                started_block_date: node.startedBlockDate.clone(),
                started_block_id: node.startedBlockId,
                started_block_timestamp: node.startedBlockTimestamp.clone(),
                destination_address: node.destinationAddress.clone(),
                refund_address: node.refundAddress.clone(),
                status: node.status.clone(),
                broker: broker_name,
            };

            match pg
                .insert_chainflip_swap_detailed(formatted_data.clone())
                .await
            {
                Ok(_) => {
                    println!(
                        "Inserted swap: id={}, date={}, src={} → dest={}",
                        node.swapRequestNativeId,
                        formatted_data.date,
                        formatted_data.source_asset,
                        formatted_data.dest_asset
                    );
                    total_inserted += 1;
                }
                Err(e) => {
                    if e.to_string()
                        .contains("duplicate key value violates unique constraint")
                    {
                        println!(
                            "Swap already exists: id={}, skipping",
                            node.swapRequestNativeId
                        );
                        total_skipped += 1;
                    } else {
                        println!("Error inserting swap {}: {:?}", node.swapRequestNativeId, e);
                    }
                }
            }

            total_fetched += 1;
        }

        // If we found existing records, we can stop fetching
        if found_existing_records {
            println!("Found records that already exist in the database, stopping fetch");
            break 'outer;
        }

        if !swaps.pageInfo.hasNextPage {
            println!("No more pages available");
            break 'outer;
        }

        offset += limit;
        println!("Moving to next page, new offset: {}", offset);
    }

    println!(
        "Incremental fetch completed. Total processed: {}, Inserted: {}, Skipped: {}",
        total_fetched, total_inserted, total_skipped
    );
    Ok(())
}
