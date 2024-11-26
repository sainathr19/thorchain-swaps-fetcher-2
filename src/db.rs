use sqlx::{postgres::PgPool, Error as SqlxError};
use std::env;
use dotenv::dotenv;

use crate::{
    models::actions_model::SwapTransactionFromatted,
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

    pub async fn insert_new_record(
        &self,
        record: SwapTransactionFromatted,
    ) -> Result<(), SqlxError> {
        let date = format_date_for_sql(&record.date).unwrap();  
        
        sqlx::query!(
            r#"
            INSERT INTO swap_history_2 (
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
            record.timestamp as i32,
            date,
            record.time,
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
            let query = "INSERT INTO swap_history_2 (
            timestamp, date, time, tx_id, 
            in_asset, in_amount, in_address, 
            out_asset_1, out_amount_1, out_address_1, 
            out_asset_2, out_amount_2, out_address_2
        ) VALUES ";
    
        let mut query_builder = sqlx::QueryBuilder::new(query);
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

    pub async fn fetch_latest_timestamp(&self) -> Result<Option<i32>, SqlxError> {
        let result: Option<i32> = sqlx::query_scalar(
            "SELECT MAX(timestamp) FROM swap_history_2"
        )
        .fetch_optional(&self.pool)
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
            ORDER BY {} {}
            LIMIT $1 OFFSET $2
            "#,
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