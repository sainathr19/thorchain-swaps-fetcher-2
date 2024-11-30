use serde::{Deserialize, Serialize};


#[derive(Serialize,Deserialize,Debug)]
pub struct ClosingPriceInterval{
    pub date : String,
    pub closing_price_usd : f64
}