use sqlx::{postgres::PgPool, Error as SqlxError};
use std::env;
use dotenv::dotenv;
use chrono;

use crate::{
    models::{actions_model::SwapTransactionFromatted, chainflip_swaps::ChainflipSwap, closing_prices::ClosingPriceInterval},
    routes::swap_history::OrderType,
    utils::{format_date_for_sql, sanitize_string},
};

#[derive(Clone)]
pub struct PostgreSQL {
    pub pool: PgPool,
}

impl PostgreSQL {
    pub async fn init() -> Result<Self, SqlxError> {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pool = PgPool::connect(&database_url).await?;
        println!("Connected to PostgreSQL");
        Ok(PostgreSQL { pool })
    }
    pub async fn insert_chainflip_swap(
        &self,
        record: ChainflipSwap,
    ) -> Result<(), SqlxError> {

        let query = r#"
            INSERT INTO chainflip_swaps (
                timestamp, date, swap_id, 
                in_asset, in_amount, in_amount_usd, in_address, 
                out_asset, out_amount, out_amount_usd, out_address
            )
            VALUES (
                $1, CAST($2 AS date), $3, 
                $4, $5, $6, $7, 
                $8, $9, $10, $11
            )
            "#;
        
        sqlx::query(query)
            .bind(record.timestamp as i32)
            .bind(record.date)
            .bind(record.swap_id)
            .bind(record.in_asset)
            .bind(record.in_amount)
            .bind(record.in_amount_usd)
            .bind(record.in_address)
            .bind(record.out_asset)
            .bind(record.out_amount)
            .bind(record.out_amount_usd)
            .bind(record.out_address)
            .execute(&self.pool)
            .await?;
    
        Ok(())
    }


    pub async fn insert_new_record(
        &self,
        table_name: &str,
        record: SwapTransactionFromatted,
    ) -> Result<(), SqlxError> {
        let date = format_date_for_sql(&record.date).unwrap();        
        let query = format!(
            r#"
            INSERT INTO {} (
                timestamp, date, time, tx_id, 
                in_asset, in_amount, in_address, 
                out_asset_1, out_amount_1, out_address_1, 
                out_asset_2, out_amount_2, out_address_2
            )
            VALUES (
                $1, $2, $3, $4, 
                $5, $6, $7, 
                $8, $9, $10, 
                $11, $12, $13
            )
            "#,
            table_name
        );
        
        sqlx::query(&query)
            .bind(record.timestamp as i32)
            .bind(date)
            .bind(record.time)
            .bind(record.tx_id)
            .bind(record.in_asset)
            .bind(record.in_amount)
            .bind(record.in_address)
            .bind(record.out_asset_1)
            .bind(record.out_amount_1)
            .bind(record.out_address_1)
            .bind(record.out_asset_2)
            .bind(record.out_amount_2)
            .bind(record.out_address_2)
            .execute(&self.pool)
            .await?;
    
        Ok(())
    }
    
    pub async fn insert_closing_price(
        &self,
        record: ClosingPriceInterval,
    ) -> Result<(), SqlxError> {
        let query = r#"
            INSERT INTO btc_closing_prices (
                date,
                closing_price_usd
            )
            VALUES ($1, $2)
            ON CONFLICT (date) DO UPDATE 
            SET closing_price_usd = EXCLUDED.closing_price_usd
        "#;

        sqlx::query(query)
            .bind(record.date)
            .bind(record.closing_price_usd)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
    pub async fn insert_bulk(
        &self,
        table_name: &str,
        records: Vec<SwapTransactionFromatted>,
    ) -> Result<(), SqlxError> {
        if records.is_empty() {
            return Ok(());
        }
    
        let query = format!(
            "INSERT INTO {} (
                timestamp, date, time, tx_id, 
                in_asset, in_amount, in_address, 
                out_asset_1, out_amount_1, out_address_1, 
                out_asset_2, out_amount_2, out_address_2
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (tx_id) DO NOTHING",
            table_name
        );
    
        let mut successful_count = 0;
        let total_records = records.len();
    
        for record in records {
            let date = format_date_for_sql(&record.date).unwrap_or_default();
            
            match sqlx::query(&query)
                .bind(record.timestamp)
                .bind(date)
                .bind(record.time)
                .bind(record.tx_id)
                .bind(sanitize_string(&record.in_asset))
                .bind(record.in_amount)
                .bind(sanitize_string(&record.in_address))
                .bind(sanitize_string(&record.out_asset_1))
                .bind(record.out_amount_1)
                .bind(sanitize_string(&record.out_address_1))
                .bind(record.out_asset_2.as_deref().map(sanitize_string))
                .bind(record.out_amount_2)
                .bind(record.out_address_2.as_deref().map(sanitize_string))
                .execute(&self.pool)
                .await 
            {
                Ok(_) => successful_count += 1,
                Err(e) => {
                    println!("Error inserting record: {:?}", e);
                    println!("Failed record");
                    // Optionally, you could log the specific record that failed
                }
            }
        }
    
        println!("Inserted {}/{} records", successful_count, total_records);
    
        Ok(())
    }

    pub async fn insert_bulk_native(
        &self,
        table_name: &str,
        records: Vec<SwapTransactionFromatted>,
    ) -> Result<(), SqlxError> {
        if records.is_empty() {
            return Ok(());
        }
        let query = format!(
            "INSERT INTO {} (
                timestamp, date, time, tx_id, 
                in_asset, in_amount, in_address, 
                out_asset_1, out_amount_1, out_address_1, 
                out_asset_2, out_amount_2, out_address_2
                    )",
            table_name
        );
    
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

    pub async fn fetch_latest_timestamp(&self, table_name: &str) -> Result<Option<i64>, SqlxError> {
        let query = format!("SELECT MAX(timestamp) FROM {}", table_name);
        let result: Option<i64> = sqlx::query_scalar(&query)
            .fetch_optional(&self.pool)
            .await?;
        
        Ok(result)
    }

    pub async fn fetch_last_timestamp(&self, table_name: &str) -> Result<Option<i64>, SqlxError> {
        let query = format!("SELECT MIN(timestamp) FROM {}", table_name);
        let result: Option<i64> = sqlx::query_scalar(&query)
            .fetch_optional(&self.pool)
            .await?;
        
        Ok(result)
    }

    pub async fn fetch_all(
        &self,
        table_name: &str,
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
            FROM {}
            WHERE (1 = 1)
            {}
            {}
            ORDER BY {} {}
            LIMIT $1 OFFSET $2
            "#,
            table_name,
            if search.is_some() {
                "AND (tx_id LIKE $3 OR in_address LIKE $4 OR out_address_1 LIKE $5 OR out_address_2 LIKE $6)"
            } else {
                ""
            },
            if let Some(_) = date {
                if search.is_some() { "AND date = $7" } else { "AND date = $3" }
            } else {
                ""
            },
            sort_by,
            match order {
                OrderType::ASC => "ASC",
                OrderType::DESC => "DESC",
            },
        );

        let mut query = sqlx::query_as::<_, SwapTransactionFromatted>(&base_query);
        query = query.bind(limit as i64).bind(offset as i64);

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
        let records = query.fetch_all(&self.pool).await?;
        Ok(records)
    }
}