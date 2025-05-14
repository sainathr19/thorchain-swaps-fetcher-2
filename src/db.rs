use dotenv::dotenv;
use sqlx::{postgres::PgPool, Error as SqlxError};
use std::env;

use crate::{
    models::{
        actions_model::SwapTransactionFromatted,
        chainflip_swaps::{ChainflipSwap, ChainflipSwapDetailed},
        closing_prices::ClosingPriceInterval,
    },
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
        table_name: &str,
    ) -> Result<(), SqlxError> {
        let query = format!(
            r#"
            INSERT INTO {} (
                timestamp, date, time, tx_id, 
                in_asset, in_amount, in_address, 
                out_asset_1, out_amount_1, out_address_1, 
                out_asset_2, out_amount_2, out_address_2
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (tx_id) DO NOTHING"#,
            table_name
        );

        let date = format_date_for_sql(&record.date).unwrap_or_default();

        sqlx::query(query.as_str().as_ref())
            .bind(record.timestamp)
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

    pub async fn fetch_latest_timestamp(&self, table_name: &str) -> Result<Option<i32>, SqlxError> {
        let query = format!("SELECT MAX(timestamp) FROM {}", table_name);
        let result: Option<i32> = sqlx::query_scalar(&query)
            .fetch_optional(&self.pool)
            .await?;

        Ok(result)
    }

    pub async fn fetch_latest_timestamp_i64(
        &self,
        table_name: &str,
    ) -> Result<Option<i64>, SqlxError> {
        let query = format!("SELECT MAX(timestamp) FROM {}", table_name);
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
                if search.is_some() {
                    "AND date = $7"
                } else {
                    "AND date = $3"
                }
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

    pub async fn insert_chainflip_swap_detailed(
        &self,
        record: ChainflipSwapDetailed,
    ) -> Result<(), SqlxError> {
        // First check if this swap already exists with the same data
        let existing_query = r#"
            SELECT id FROM chainflip_swaps_detailed 
            WHERE swap_id = $1 
        "#;

        let _existing = sqlx::query(existing_query)
            .bind(&record.swap_id)
            .fetch_optional(&self.pool)
            .await?;

        // If record exists, perform an update, otherwise insert
        let query = r#"
            INSERT INTO chainflip_swaps_detailed (
                timestamp, date, swap_id, 
                source_asset, dest_asset, 
                base_asset_leg1, base_asset_leg2,
                ingress_amount, ingress_value_usd,
                input_amount, input_value_usd,
                output_amount, output_value_usd,
                started_block_date, started_block_id, started_block_timestamp,
                destination_address, refund_address,
                status, broker
            )
            VALUES (
                $1, CAST($2 AS date), $3, 
                $4, $5, $6, $7, 
                $8, $9, $10, $11,
                $12, $13, $14, $15, $16,
                $17, $18, $19, $20
            )
            ON CONFLICT (swap_id) DO UPDATE 
            SET 
                timestamp = EXCLUDED.timestamp,
                date = EXCLUDED.date,
                source_asset = EXCLUDED.source_asset,
                dest_asset = EXCLUDED.dest_asset,
                base_asset_leg1 = EXCLUDED.base_asset_leg1,
                base_asset_leg2 = EXCLUDED.base_asset_leg2,
                ingress_amount = EXCLUDED.ingress_amount,
                ingress_value_usd = EXCLUDED.ingress_value_usd,
                input_amount = EXCLUDED.input_amount,
                input_value_usd = EXCLUDED.input_value_usd,
                output_amount = EXCLUDED.output_amount,
                output_value_usd = EXCLUDED.output_value_usd,
                started_block_date = EXCLUDED.started_block_date,
                started_block_id = EXCLUDED.started_block_id,
                started_block_timestamp = EXCLUDED.started_block_timestamp,
                destination_address = EXCLUDED.destination_address,
                refund_address = EXCLUDED.refund_address,
                status = EXCLUDED.status,
                broker = EXCLUDED.broker
            "#;

        sqlx::query(query)
            .bind(record.timestamp as i32)
            .bind(record.date)
            .bind(record.swap_id)
            .bind(record.source_asset)
            .bind(record.dest_asset)
            .bind(record.base_asset_leg1)
            .bind(record.base_asset_leg2)
            .bind(record.ingress_amount)
            .bind(record.ingress_value_usd)
            .bind(record.input_amount)
            .bind(record.input_value_usd)
            .bind(record.output_amount)
            .bind(record.output_value_usd)
            .bind(record.started_block_date)
            .bind(record.started_block_id)
            .bind(record.started_block_timestamp)
            .bind(record.destination_address)
            .bind(record.refund_address)
            .bind(record.status)
            .bind(record.broker)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
