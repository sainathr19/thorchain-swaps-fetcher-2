pub mod coingecko;
pub mod cron;
pub mod midgard;
pub mod transaction_handler;

use chrono::{NaiveDate, ParseError, TimeZone, Utc};
use regex::Regex;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::num::{ParseFloatError, ParseIntError};
use std::path::Path;

pub fn convert_to_standard_unit(amount: f64, decimals: u32) -> f64 {
    let divisor = 10u64.pow(decimals);
    amount as f64 / divisor as f64
}

pub fn calculate_transaction_amount(amount: f64, price: f64) -> f64 {
    amount * price
}
pub fn convert_nano_to_sec(nano_str: &str) -> String {
    let nanoseconds: i64 = nano_str.parse().expect("Invalid date string");
    let seconds = nanoseconds / 1_000_000_000;
    seconds.to_string()
}

pub fn parse_f64(input: &str) -> Result<f64, ParseFloatError> {
    input.parse::<f64>().map_err(|e| e)
}

pub fn parse_u64(input: &str) -> Result<u64, ParseIntError> {
    input.parse::<u64>().map_err(|e| e)
}

pub fn format_epoch_timestamp(epoch_nanos: &str) -> Result<(String, String), Box<dyn Error>> {
    let nanos = epoch_nanos.parse::<i64>()?;
    let seconds = nanos / 1_000_000_000;
    let nanoseconds = (nanos % 1_000_000_000) as u32;

    let datetime = Utc
        .timestamp_opt(seconds, nanoseconds)
        .single()
        .ok_or("Invalid timestamp")?;

    let formatted_date = datetime.format("%d-%m-%Y").to_string();
    let formatted_time = datetime.format("%I:%M%p").to_string().to_lowercase();

    Ok((formatted_date, formatted_time))
}

pub fn coin_name_from_pool(pool_name: &str) -> Option<String> {
    let re = Regex::new(r"[./~\-_|:;,+*^$!?]").unwrap();
    let parts: Vec<&str> = re.split(pool_name).collect();
    parts.get(1).map(|s| s.to_string())
}

pub fn asset_name_from_pool(pool_name: &str) -> Option<String> {
    let re = Regex::new(r"[./~\-_|:;,+*^$!?]").unwrap();
    let mut parts = re.split(pool_name);
    match (parts.next(), parts.next()) {
        (Some(first), Some(second)) => Some(format!("{}.{}", first, second)),
        _ => None,
    }
}

pub fn format_date_for_sql(date_str: &str) -> Result<String, ParseError> {
    let date = NaiveDate::parse_from_str(date_str, "%d-%m-%Y")?;
    Ok(date.format("%Y-%m-%d").to_string())
}

const TOKEN_FILE_PATH: &str = "next_page_token.txt";

pub fn read_next_page_token_from_file() -> io::Result<String> {
    if Path::new(TOKEN_FILE_PATH).exists() {
        fs::read_to_string(TOKEN_FILE_PATH).map(|token| token.trim().to_string())
    } else {
        Ok(String::from("170981189000000012"))
    }
}

pub fn write_next_page_token_to_file(token: &str) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(TOKEN_FILE_PATH)?;
    file.write_all(token.as_bytes())?;
    Ok(())
}
