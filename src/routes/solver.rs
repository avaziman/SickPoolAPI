use crate::SickApiData;

use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use redis::Commands;
use redis_ts::{AsyncTsCommands, TsFilterOptions, TsRange, TsMget};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize)]
pub struct Solver {
    pub address: String,
    pub hashrate: f64,
    pub worker_count: u32,
    pub balance: f64,
    pub joined: i64,
}

#[derive(Deserialize)]
pub struct SolverQuery {
    pub address: std::option::Option<String>,
    pub id: std::option::Option<String>,
}

#[derive(Serialize, Clone)]
struct BalanceEntry {
    time: u64,
    balance: f64,
}

pub fn solver_route(cfg: &mut web::ServiceConfig) {
    cfg.service(balance);
}

#[get("/balance")]
async fn balance(
    req: HttpRequest,
    info: web::Query<SolverQuery>,
    api_data: web::Data<SickApiData>,
) -> impl Responder {
    let mut con = api_data.redis.clone();

    let tsType = "immature-balance";

    let filter: TsFilterOptions = if info.address.is_some() {
        TsFilterOptions::default()
            .equals("type", tsType)
            .equals("address", info.address.as_ref().unwrap())
    } else if info.id.is_some() {
        TsFilterOptions::default()
            .equals("type", tsType)
            .equals("id", info.id.as_ref().unwrap())
    } else {
        return HttpResponse::BadRequest().body("No address or id provided.");
    };

    let immature_balance_tms: TsMget<u64,f64> = match con.ts_mget(filter).await {
        Ok(res) => res,
        Err(err) => {
            eprintln!("range query error: {}", err);
            return HttpResponse::NotFound().body(
                json!({
                    "error": "Key not found",
                    "result": Value::Null
                })
                .to_string(),
            );
        }
    };

    let immature_balance = immature_balance_tms.values.first().unwrap().value.unwrap();
    // let immate_balance = match immature_balance_tms.values().first().value(){
    //     Some(res) => res,
    //     None => (0,0)
    // };

    return HttpResponse::Ok().body(
        json!({
            "error": Value::Null,
            "result": {"immature": immature_balance.1}
        })
        .to_string(),
    );
}

// #[get("/balanceHistory")]
// async fn balance_history(
//     req: HttpRequest,
//     info: web::Query<SolverQuery>,
//     api_data: web::Data<SickApiData>,
// ) -> impl Responder {
//     let mut con = api_data.redis.clone();

//     let filter: TsFilterOptions = if info.address.is_some() {
//         TsFilterOptions::default()
//             .equals("type", tsType)
//             .equals("address", info.address.as_ref().unwrap())
//     } else if info.id.is_some() {
//         TsFilterOptions::default()
//             .equals("type", tsType)
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
