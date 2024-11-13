use actix_web::{
    post,
    web::{self, ServiceConfig},
    HttpResponse, Responder,
};
use serde::{Deserialize, Serialize};

use crate::{db::MySQL, utils::parse_u64};

#[derive(Serialize, Deserialize, Debug)]
pub enum OrderType {
    ASC,
    DESC,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct RequestBody {
    sort_by: String,
    page: String,
    limit: String,
    order: String,
    search: Option<String>,
    date: Option<String>,
}
#[post("/swaps")]
pub async fn swap_history(
    mysql: web::Data<MySQL>,
    options: web::Json<RequestBody>,
) -> impl Responder {
    let options = options.into_inner();
    let order;
    if options.order == "ASC" {
        order = OrderType::ASC;
    } else {
        order = OrderType::DESC;
    }
    let page = parse_u64(&options.page).unwrap();
    let limit = parse_u64(&options.limit).unwrap();
    let offset: u64 = (page - 1) * limit;
    let records = mysql
        .fetch_all(
            order,
            limit,
            options.sort_by,
            offset,
            options.search,
            options.date,
        )
        .await;
    match records {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(err) => {
            println!("{:?}", err);
            HttpResponse::BadRequest().json("Error Fetching Data")
        }
    }
}

pub fn init(config: &mut ServiceConfig) {
    config.service(swap_history);
}
