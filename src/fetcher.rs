use crate::db::MySQL;
use crate::models::actions_model::SwapTransactionFromatted;
use crate::utils::midgard::MidGard;
use crate::utils::transaction_handler::{TransactionError, TransactionHandler};
use crate::utils::{read_next_page_token_from_file, write_next_page_token_to_file};
use chrono::Utc;

pub async fn fetch_historical_data() -> Result<(), TransactionError> {
    let mysql = MySQL::init().await.map_err(|e| {
        TransactionError::DatabaseError(format!("Error connecting to MySQL: {:?}", e))
    })?;
    const TOKEN_FILE_PATH: &str = "next_page_token.txt";
    let mut next_page_token = read_next_page_token_from_file(TOKEN_FILE_PATH).unwrap_or_default();

    loop {
        let resp = match MidGard::fetch_actions_with_nextpage(next_page_token.as_str()).await {
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

        let process_response =
            TransactionHandler::process_and_insert_transaction(&mysql, &resp.actions).await;

        match process_response {
            Ok(_) => {
                next_page_token = resp.meta.nextPageToken.clone();
                if let Err(e) = write_next_page_token_to_file(&next_page_token,TOKEN_FILE_PATH) {
                    return Err(TransactionError::FileError(format!(
                        "Error writing next page token to file: {:?}",
                        e
                    )));
                }
                println!("Updated next page token: {}", &next_page_token);
            }
            Err(err) => {
                return Err(TransactionError::ProcessingError(format!(
                    "Error processing transaction: {:?}",
                    err
                )));
            }
        }
    }

    Ok(())
}

pub async fn fetch_latest_data(mysql: &MySQL) -> Result<(), TransactionError> {
    let latest_timestamp = match mysql.fetch_latest_timestamp().await {
        Ok(Some(timestamp)) => timestamp,
        Ok(None) => Utc::now().timestamp() as i64,
        Err(err) => {
            return Err(TransactionError::DatabaseError(format!(
                "Error fetching the latest timestamp: {:?}",
                err
            )));
        }
    };

    let mysql_clone = mysql.clone();
    let latest_timestamp_str = latest_timestamp.to_string();

    // Fetch actions with the latest timestamp
    let mut resp = match MidGard::fetch_actions_with_timestamp(&latest_timestamp_str).await {
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
        TransactionHandler::process_and_insert_transaction(&mysql_clone, &actions).await;
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
        resp = match MidGard::fetch_actions_with_prevpage(prev_page_token.as_str()).await {
            Ok(response) => response,
            Err(err) => {
                return Err(TransactionError::ApiError(format!(
                    "Error fetching previous page actions: {:?}",
                    err
                )));
            }
        };

        let process_response =
            TransactionHandler::process_and_insert_transaction(&mysql_clone, &resp.actions).await;
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


pub async fn fetch_from_start(mysql: &MySQL) -> Result<(), TransactionError> {
    let mysql_clone = mysql.clone();
    let start_timestamp = "1700357476";
    const TOKEN_FILE_PATH: &str = "prev_page_token.txt";
    
    let stored_token = read_next_page_token_from_file(TOKEN_FILE_PATH).unwrap_or_default();
    
    let mut resp = if stored_token.is_empty() {
        match MidGard::fetch_actions_with_timestamp(&start_timestamp).await {
            Ok(response) => {
                let actions = response.actions.clone();
                match TransactionHandler::process_and_insert_transaction(&mysql_clone, &actions).await {
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
        match MidGard::fetch_actions_with_prevpage(&stored_token).await {
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
        resp = match MidGard::fetch_actions_with_prevpage(&prev_page_token).await {
            Ok(response) => response,
            Err(err) => {
                return Err(TransactionError::ApiError(format!(
                    "Error fetching previous page actions: {:?}",
                    err
                )));
            }
        };
        let processed_transactions = TransactionHandler::process_transactions(&resp.actions).await;
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
            let insertion_response = mysql.insert_bulk(transaction_batch.clone()).await;
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