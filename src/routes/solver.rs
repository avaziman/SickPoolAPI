use crate::routes::redis::key_format;
use crate::SickApiData;

use super::table_res::TableRes;
use crate::redis_interop::ffi::Prefix;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use mysql::prelude::*;
use mysql::*;
use redis::Commands;
use redis_ts::{AsyncTsCommands, TsFilterOptions, TsMget, TsRange};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize)]
pub struct Solver {
    pub id: u32,
    pub address: String,
    pub hashrate: f64,
    pub round_effort: f64,
    // pub worker_count: u32,
    pub balance: u64,
    pub joined: u64,
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
    cfg.service(solver_overview);
}

fn addr_not_found() -> HttpResponse {
    HttpResponse::NotFound().body(json!({"error": "Address not found!"}).to_string())
}

#[get("/overview")]
async fn solver_overview(
    info: web::Query<OverviewQuery>,
    api_data: web::Data<SickApiData>,
) -> impl Responder {
    let mut con = api_data.redis.clone();
    let mut con_mysql = api_data.mysql.get_conn().unwrap();

    // lowercase or id_tag@ to valid address
    let stmt = con_mysql
        .prep("SELECT address,alias FROM addresses WHERE address = ? OR alias = ?")
        .unwrap();
    let (address, alias): (String, Option<String>) =
        match con_mysql.exec_first(&stmt, (&info.address, &info.address)) {
            Ok(res) => match res {
                Some(re) => re,
                None => {
                    return addr_not_found();
                }
            },
            Err(e) => {
                return addr_not_found();
            }
        };

    let (immature_balance, mature_balance): (u64, u64) = (0, 0);


    HttpResponse::Ok().body(
        json!({
            "error": Value::Null,
            "result": {
                "address": address,
                "balance": {"mature": mature_balance, "immature": immature_balance},
                "alias": alias
            }
        })
        .to_string(),
    )
}

pub fn miner_id_filter(addr: &String) -> TsFilterOptions {
    if addr.ends_with('@') {
        TsFilterOptions::default().equals("identity", addr)
    } else {
        TsFilterOptions::default().equals("address", addr.to_ascii_lowercase())
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
