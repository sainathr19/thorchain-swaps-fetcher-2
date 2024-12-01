pub mod coingecko;
pub mod cron;
pub mod midgard;
pub mod chainflip;
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

pub fn asset_name_from_pool(pool_name: &str) -> Option<String> {
    let re = Regex::new(r"[./~\-_|:;,+*^$!?]").unwrap();
    let mut parts = re.split(pool_name);
    match (parts.next(), parts.next()) {
        (Some(first), Some(second)) => Some(format!("{}.{}", first, second)),
        _ => None,
    }
}

pub fn asset_name_from_trade_pool(pool_name: &str) -> Option<String> {
    let delimiters = r"[./~]";
    let re = Regex::new(delimiters).unwrap();
    let mut parts = re.split(pool_name);
    let delimiter_match = re.find(pool_name).map(|m| m.as_str()); // Find the first matched delimiter.

    match (parts.next(), parts.next(), delimiter_match) {
        (Some(first), Some(second), Some(delimiter)) => Some(format!("{}{}{}", first, delimiter, second)),
        _ => None,
    }
}

pub fn format_date_for_sql(date_str: &str) -> Result<String, ParseError> {
    let date = NaiveDate::parse_from_str(date_str, "%d-%m-%Y")?;
    Ok(date.format("%Y-%m-%d").to_string())
}

pub fn read_next_page_token_from_file(file_path : &str) -> io::Result<String> {
    if Path::new(file_path).exists() {
        fs::read_to_string(file_path).map(|token| token.trim().to_string())
    } else {
        Ok(String::from(""))
    }
}

pub fn write_next_page_token_to_file(token: &str, file_path : &str) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_path)?;
    file.write_all(token.as_bytes())?;
    Ok(())
}

pub fn sanitize_string(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_ascii() || c.is_alphanumeric())
        .collect::<String>()
}
