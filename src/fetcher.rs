use std::collections::HashSet;
use std::sync::Arc;

use crate::db::PostgreSQL;
use crate::models::actions_model::SwapTransactionFromatted;
use crate::utils::midgard::MidGard;
use crate::utils::transaction_handler::{TransactionError, TransactionHandler};
use crate::utils::{read_next_page_token_from_file, write_next_page_token_to_file};
use crate::SwapType;
use chrono::Utc;
use futures_util::lock::Mutex;
pub async fn fetch_historical_data(base_url: &str,swap_type: SwapType) -> Result<(), TransactionError> {
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
                SwapType::NATIVE => "btc_user_data",
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
        SwapType::NATIVE => "btc_user_data",
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
pub async fn fetch_from_start(pg: &PostgreSQL,base_url: &str,swap_type: SwapType) -> Result<(), TransactionError> {
    let transaction_handler = TransactionHandler;

    let pg_clone = pg.clone();
    let start_timestamp = "1700357476";
    const TOKEN_FILE_PATH: &str = "prev_page_token.txt";
    
    let stored_token = read_next_page_token_from_file(TOKEN_FILE_PATH).unwrap_or_default();
    
    let mut resp = if stored_token.is_empty() {
        match MidGard::fetch_actions_with_timestamp(base_url,&start_timestamp).await {
            Ok(response) => {
                let actions = response.actions.clone();
                match transaction_handler.process_and_insert_transaction(&pg_clone, &actions,swap_type.clone()).await {
                    Ok(_) => {
                        if let Err(e) = write_next_page_token_to_file(&response.meta.prevPageToken, TOKEN_FILE_PATH) {
                            return Err(TransactionError::FileError(format!(
                                "Error writing next page token to file: {:?}",
                                e
                            )));
                        }
                        println!("Updated Prev page token: {}", &response.meta.prevPageToken);
                        response
                    }
                    Err(err) => {
                        return Err(TransactionError::ProcessingError(format!(
                            "Error processing transaction: {:?}",
                            err
                        )));
                    }
                }
            }
            Err(err) => {
                return Err(TransactionError::ApiError(format!(
                    "Error fetching actions with timestamp: {:?}",
                    err
                )));
            }
        }
    } else {
        match MidGard::fetch_actions_with_prevpage(base_url,&stored_token).await {
            Ok(response) => response,
            Err(err) => {
                return Err(TransactionError::ApiError(format!(
                    "Error fetching previous page actions: {:?}",
                    err
                )));
            }
        }
    };

    let mut transaction_batch : Vec<SwapTransactionFromatted> = Vec::new();
    let mut batch_count = 0;
    while !resp.actions.is_empty() {
        let prev_page_token = resp.meta.prevPageToken.clone();
        resp = match MidGard::fetch_actions_with_prevpage(base_url,&prev_page_token).await {
            Ok(response) => response,
            Err(err) => {
                return Err(TransactionError::ApiError(format!(
                    "Error fetching previous page actions: {:?}",
                    err
                )));
            }
        };
        let processed_transactions = transaction_handler.process_transactions(&resp.actions,swap_type.clone()).await;
        match processed_transactions{
            Ok(val)=>{
                transaction_batch.extend(val);
                batch_count+=1;
                println!("Processed Batch : {}",&batch_count);
                if let Err(e) = write_next_page_token_to_file(&prev_page_token, TOKEN_FILE_PATH) {
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
                SwapType::NATIVE => "btc_user_data",
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
    }

    Ok(())
}