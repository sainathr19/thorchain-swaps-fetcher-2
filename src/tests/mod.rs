#[cfg(test)]
mod tests {
    use crate::utils::{
        asset_name_from_pool, calculate_transaction_amount, coin_name_from_pool,
        convert_nano_to_sec, convert_to_standard_unit, format_date_for_sql, parse_f64, parse_u64,
        read_next_page_token_from_file, write_next_page_token_to_file,
    };

    use std::fs;

    #[test]
    fn test_convert_to_standard_unit() {
        assert_eq!(convert_to_standard_unit(1000000.0, 6), 1.0);
        assert_eq!(convert_to_standard_unit(123456789.0, 6), 123.456789);
    }

    #[test]
    fn test_calculate_transaction_amount() {
        assert_eq!(calculate_transaction_amount(10.0, 2.5), 25.0);
        assert_eq!(calculate_transaction_amount(5.0, 3.0), 15.0);
    }

    #[test]
    fn test_convert_nano_to_sec() {
        assert_eq!(convert_nano_to_sec("1000000000"), "1");
        assert_eq!(convert_nano_to_sec("5000000000"), "5");
    }

    #[test]
    fn test_parse_f64() {
        assert_eq!(parse_f64("123.45").unwrap(), 123.45);
        assert!(parse_f64("abc").is_err());
    }

    #[test]
    fn test_parse_u64() {
        assert_eq!(parse_u64("123456").unwrap(), 123456);
        assert!(parse_u64("xyz").is_err());
    }

    #[test]
    fn test_coin_name_from_pool() {
        assert_eq!(coin_name_from_pool("pool.btc~usd"), Some("btc".to_string()));
        assert_eq!(coin_name_from_pool("pool-eth-usd"), Some("eth".to_string()));
        assert_eq!(coin_name_from_pool("poolname"), None);
    }

    #[test]
    fn test_asset_name_from_pool() {
        assert_eq!(asset_name_from_pool("btc.eth"), Some("btc.eth".to_string()));
        assert_eq!(
            asset_name_from_pool("usd-btc~eth"),
            Some("usd.btc".to_string())
        );
        assert_eq!(asset_name_from_pool("name"), None);
    }

    #[test]
    fn test_format_date_for_sql() {
        assert_eq!(format_date_for_sql("14-08-2023").unwrap(), "2023-08-14");
        assert!(format_date_for_sql("invalid-date").is_err());
    }

    const TOKEN_FILE_PATH: &str = "next_page_token.txt";

    #[test]
    fn test_read_next_page_token_from_file() {
        let token = read_next_page_token_from_file().unwrap_or("default".to_string());
        assert!(token.is_empty() || token == "170981189000000012" || token == "default");
    }

    #[test]
    fn test_write_next_page_token_to_file() {
        let test_token = "test_token";
        write_next_page_token_to_file(test_token).unwrap();

        let read_token = read_next_page_token_from_file().unwrap();
        assert_eq!(read_token, test_token);
        fs::remove_file(TOKEN_FILE_PATH).unwrap();
    }
}
