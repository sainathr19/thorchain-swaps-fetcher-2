use dotenv::dotenv;
use sqlx::{mysql::MySqlPool, Error as SqlxError};
use std::env;

use crate::{
    models::actions_model::SwapTransactionFromatted,
    routes::swap_history::OrderType,
    utils::{format_date_for_sql, sanitize_string},
};

#[derive(Clone)]
pub struct MySQL {
    pub pool: MySqlPool,
}

impl MySQL {
    pub async fn init() -> Result<Self, SqlxError> {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pool = MySqlPool::connect(&database_url).await?;
        println!("Connected to MySQL");
        Ok(MySQL { pool })
    }

    pub async fn insert_new_record(
        &self,
        record: SwapTransactionFromatted,
    ) -> Result<(), SqlxError> {
        // Parsing and formatting the data
        let date = format_date_for_sql(&record.date).unwrap();
        let time = record.time.clone();

        // Executing the insert query
        sqlx::query!(
            r#"
            INSERT INTO swap_history_2 (
                timestamp, date, time, tx_id, 
                in_asset, in_amount, in_address, 
                out_asset_1, out_amount_1, out_address_1, 
                out_asset_2, out_amount_2, out_address_2
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            record.timestamp,
            date,
            time,
            record.tx_id,
            record.in_asset,
            record.in_amount,
            record.in_address,
            record.out_asset_1,
            record.out_amount_1,
            record.out_address_1,
            record.out_asset_2,
            record.out_amount_2,
            record.out_address_2
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn insert_bulk(
        &self,
        records: Vec<SwapTransactionFromatted>,
    ) -> Result<(), SqlxError> {
        if records.is_empty() {
            return Ok(());
        }
    
        // Remove 'VALUES' from the initial query string since QueryBuilder will add it
        let query = "INSERT INTO swap_history_2 (
            timestamp, date, time, tx_id, 
            in_asset, in_amount, in_address, 
            out_asset_1, out_amount_1, out_address_1, 
            out_asset_2, out_amount_2, out_address_2
        )";
    
        let mut query_builder = sqlx::QueryBuilder::new(query);
        
        // QueryBuilder will automatically add the VALUES keyword
        query_builder.push_values(records, |mut b, record| {
            let date = format_date_for_sql(&record.date).unwrap_or_default();
            
            b.push_bind(record.timestamp)
             .push_bind(date)
             .push_bind(record.time)
             .push_bind(record.tx_id)
             .push_bind(sanitize_string(&record.in_asset))
             .push_bind(record.in_amount)
             .push_bind(sanitize_string(&record.in_address))
             .push_bind(sanitize_string(&record.out_asset_1))
             .push_bind(record.out_amount_1)
             .push_bind(sanitize_string(&record.out_address_1))
             .push_bind(record.out_asset_2.as_deref().map(sanitize_string))
             .push_bind(record.out_amount_2)
             .push_bind(record.out_address_2.as_deref().map(sanitize_string));
        });
    
        match query_builder.build().execute(&self.pool).await {
            Ok(_) => Ok(()),
            Err(e) => {
                println!("Error executing query: {:?}", e);
                println!("Query: {}", query_builder.sql());
                Err(e)
            }
        }
    }
    pub async fn fetch_latest_timestamp(&self) -> Result<Option<i64>, SqlxError> {
        let result = sqlx::query_scalar!(
            r#"
            SELECT MAX(timestamp) as "timestamp: i64"
            FROM swap_history_2
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(result)
    }

    pub async fn fetch_all(
        &self,
        order: OrderType,
        limit: u64,
        sort_by: String,
        offset: u64,
        search: Option<String>,
        date: Option<String>,
    ) -> Result<Vec<SwapTransactionFromatted>, SqlxError> {
        let base_query = format!(
            r#"
            SELECT 
                timestamp, date, time, tx_id, 
                in_asset, in_amount, in_address,
                out_asset_1, out_amount_1, out_address_1,
                out_asset_2, out_amount_2, out_address_2
            FROM transactions
            WHERE (1 = 1)
            {}
            {}
            ORDER BY {} {:?}
            LIMIT ? OFFSET ?
            "#,
            if search.is_some() {
                "AND (tx_id LIKE ? OR in_address LIKE ? OR out_address_1 LIKE ? OR out_address_2 LIKE ?)"
            } else {
                ""
            },
            if let Some(_) = date {
                "AND date = ?"
            } else {
                ""
            },
            sort_by,
            order,
        );

        let mut query = sqlx::query_as::<_, SwapTransactionFromatted>(&base_query);

        if let Some(search_term) = search {
            let search_pattern = format!("%{}%", search_term);
            query = query
                .bind(search_pattern.clone())
                .bind(search_pattern.clone())
                .bind(search_pattern.clone())
                .bind(search_pattern.clone());
        }

        if let Some(date_value) = date {
            query = query.bind(date_value);
        }
        
        query = query.bind(limit as i64).bind(offset as i64);

        let records = query.fetch_all(&self.pool).await?;

        Ok(records)
    }
}