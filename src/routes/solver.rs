use crate::SickApiData;

use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use redis::Commands;
use redis_ts::{AsyncTsCommands, TsFilterOptions, TsMget, TsRange};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use super::table_res::TableRes;

#[derive(Debug, Serialize)]
pub struct Solver {
    pub address: String,
    pub hashrate: f64,
    pub worker_count: u32,
    pub balance: f64,
    pub joined: i64,
}

#[derive(Deserialize)]
pub struct OverviewQuery {
    pub coin: String,
    pub address: String,
}

#[derive(Serialize, Clone)]
struct BalanceEntry {
    time: u64,
    balance: f64,
}

pub fn solver_route(cfg: &mut web::ServiceConfig) {
    cfg.service(overview);
}

#[get("/overview")]
async fn overview(
    info: web::Query<OverviewQuery>,
    api_data: web::Data<SickApiData>,
) -> impl Responder {
    let mut con = api_data.redis.clone();

    // lowercase or id_tag@ to valid address
    let key = info.coin.clone() + ":address-map";

    let field_addr = if info.address.ends_with('@'){
        info.address.clone()
    }else {
        info.address.to_lowercase()
    };

    let address: String = match redis::cmd("HGET")
        .arg(key)
        .arg(&field_addr)
        .query_async(&mut con)
        .await
    {
        Ok(r) => r,
        Err(err) => {
            return HttpResponse::NotFound().body(
                json!({
                    "error": "Key not found",
                    "result": Value::Null
                })
                .to_string(),
            );
        }
    };

    let solver_key = info.coin.clone() + ":solver:" + &address;

    let (immature_balance, mature_balance, identity): (
        u64,
        u64,
        Option<String>,
    ) = match redis::cmd("HMGET")
        .arg(&solver_key)
        .arg("immature-balance")
        .arg("mature-balance")
        .arg("identity")
        .query_async(&mut con)
        .await
    {
        Ok(r) => r,
        Err(err) => {
            eprintln!("Overview redis err: {}", err);
            return HttpResponse::NotFound().body(
                json!({
                    "error": "Failed to get overview",
                    "result": Value::Null
                })
                .to_string(),
            );
        }
    };

    HttpResponse::Ok().body(
        json!({
            "error": Value::Null,
            "result": {"address": address, "balance": {"mature": mature_balance, "immature": immature_balance}, "identity": identity}
        })
        .to_string(),
    )
}

pub fn miner_id_filter(addr: &String) -> TsFilterOptions{
    if addr.ends_with('@'){
        TsFilterOptions::default().equals("identity", addr)
    }else {
        TsFilterOptions::default().equals("address", addr)
    }
}


// #[get("/balanceHistory")]
// async fn balance_history(
//     req: HttpRequest,
//     info: web::Query<OverviewQuery>,
//     api_data: web::Data<SickApiData>,
// ) -> impl Responder {
//     let mut con = api_data.redis.clone();

//     let filter: TsFilterOptions = if info.address.is_some() {
//         TsFilterOptions::default()
//             .equals("type", ts_type)
//             .equals("address", info.address.as_ref().unwrap())
//     } else if info.id.is_some() {
//         TsFilterOptions::default()
//             .equals("type", ts_type)
//             .equals("id", info.id.as_ref().unwrap())
//     } else {
//         return HttpResponse::BadRequest().body("No address or id provided.");
//     };

//     let tms: TsRange<u64, f64> = match con.ts_range(key, 0, "+", None::<usize>, None).await {
//         Ok(res) => res,
//         Err(err) => {
//             eprintln!("range query error: {}", err);
//             return HttpResponse::NotFound().body(
//                 json!({
//                     "error": "Key not found",
//                     "result": Value::Null
//                 })
//                 .to_string(),
//             );
//         }
//     };

//     let mut res_vec: Vec<BalanceEntry> = Vec::new();
//     res_vec.reserve(tms.values.len());

//     // timeserieses are sorted by alphabetical order
//     for (i, el) in tms.values.iter().enumerate() {
//         res_vec.push(BalanceEntry {
//             time: el.0,
//             balance: el.1 as f64,
//         });
//     }

//     HttpResponse::Ok().body(
//         json!({
//             "error": Value::Null,
//             "result": res_vec
//         })
//         .to_string(),
//     )
// }
