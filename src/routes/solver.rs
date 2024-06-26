use crate::routes::redis::key_format;
use crate::SickApiData;

use crate::redis_interop::ffi::Prefix;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use mysql::prelude::*;
use mysql::*;
use redis::Commands;
use redis_ts::{AsyncTsCommands, TsFilterOptions, TsMget, TsRange};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Solver {
    pub id: u32,
    pub hashrate: f64,
    pub round_effort: f64,
    // pub worker_count: u32,
    #[serde(flatten)]
    pub info: SolverInfo
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SolverInfo {
    pub address: String,
    pub joined: u64,
    pub mature_balance: u64,
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
    mut info: web::Query<OverviewQuery>,
    api_data: web::Data<SickApiData>,
) -> impl Responder {
    let mut con_mysql = api_data.mysql.get_conn().unwrap();

    if info.address.starts_with('@'){
        info.address.remove(0); 
    }else {
        info.address = info.address.to_lowercase(); 
    }

    // lowercase or id_tag@ to valid address
    let stmt = con_mysql
        .prep("CALL GetMinerOverview(?)")
        .unwrap();

    let (address, alias, mature_balance, immature_balance): (String, Option<String>, u64, u64) =
        match con_mysql.exec_first(&stmt, (&info.address,)) {
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

pub fn miner_alias_filter(addr: &String) -> TsFilterOptions {
    if addr.starts_with('@') {
        TsFilterOptions::default().equals("alias", &addr[1..addr.len()])
    } else {
        TsFilterOptions::default().equals("address", addr.to_ascii_lowercase())
    }
}