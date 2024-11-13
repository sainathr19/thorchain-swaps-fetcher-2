use dotenv::dotenv;
use sqlx::{mysql::MySqlPool, Error as SqlxError};
use std::{env, fmt};

use crate::{
    models::actions_model::SwapTransactionFromatted, routes::swap_history::OrderType,
    utils::format_date_for_sql,
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
            INSERT INTO swap_history (timestamp, date, time, tx_id, in_asset, in_amount, in_amount_usd, in_address, out_asset_1, out_amount_1, out_amount_1_usd, out_address_1, out_asset_2, out_amount_2, out_amount_2_usd, out_address_2)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            record.timestamp,
            date,
            time,
            record.tx_id,
            record.in_asset,
            record.in_amount,
            record.in_amount_usd,
            record.in_address,
            record.out_asset_1,
            record.out_amount_1,
            record.out_amount_1_usd,
            record.out_address_1,
            record.out_asset_2,
            record.out_amount_2,
            record.out_amount_2_usd,
            record.out_address_2
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn fetch_latest_timestamp(&self) -> Result<Option<i64>, SqlxError> {
        let result = sqlx::query_scalar!(
            r#"
            SELECT MAX(timestamp) as "timestamp: i64"
            FROM swap_history
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
            SELECT timestamp, date, time, tx_id, in_asset, in_amount, in_amount_usd, in_address,
                   out_asset_1, out_amount_1, out_amount_1_usd, out_address_1,
                   out_asset_2, out_amount_2, out_amount_2_usd, out_address_2
            FROM swap_history
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
