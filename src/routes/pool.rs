use crate::SickApiData;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
extern crate redis;

use redis::aio::ConnectionManager;
use redis::{AsyncCommands, FromRedisValue};
use redis_ts::{
    AsyncTsCommands, TsAggregationType, TsFilterOptions, TsMget, TsMrange, TsOptions, TsRange,
};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use std::arch::x86_64::_mm_comineq_sd;
use std::collections::HashMap;

use super::block::Block;
use super::payout::FinishedPayment;
use super::solver::{solver_overview, Solver};
use super::table_res::TableRes;

// use redisearch_api::{init, Document, FieldType, Index, TagOptions};
use serde::Deserialize;

#[derive(Deserialize)]
struct CoinQuery {
    coin: String,
}

#[derive(Deserialize)]
struct TableQuerySort {
    coin: String,
    page: u32,
    limit: u8,
    sortby: String,
    sortdir: String,
    // chains: Vec<String>,
}
#[derive(Deserialize)]
struct TableQuery {
    coin: String,
    page: u32,
    limit: u8,
}

pub fn pool_route(cfg: &mut web::ServiceConfig) {
    cfg.service(pool_overview);
    cfg.service(staking_balance);
    cfg.service(block_number);
    cfg.service(current_effort_pow);
    cfg.service(blocks);
    cfg.service(payouts);
    cfg.service(solvers);
    cfg.service(hashrate_history);
}

#[get("/hashrateHistory")]
async fn hashrate_history(
    api_data: web::Data<SickApiData>,
    info: web::Query<CoinQuery>,
) -> impl Responder {
    let mut con = api_data.redis.clone();

    let key = info.coin.clone() + ":hashrate:pool";

    let tms: TsRange<u64, f64> = match con.ts_range(key, 0, "+", None::<usize>, None).await {
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

    let mut res_vec: Vec<(u64, f64)> = Vec::new();
    for (i, el) in tms.values.iter().enumerate() {
        // time in unix seconds not ms to save bandwidth
        res_vec.push((el.0 / 1000, el.1));
    }

    HttpResponse::Ok().body(
        json!({
            "error": Value::Null,
            "result": res_vec
        })
        .to_string(),
    )
}

#[get("/overview")]
async fn pool_overview(
    info: web::Query<CoinQuery>,
    api_data: web::Data<SickApiData>,
) -> impl Responder {
    let mut con = api_data.redis.clone();

    let pool_hashrate_key = format!("{}:hashrate:pool", &info.coin);
    let net_hashrate_key = format!("{}:{}:network-hashrate", &info.coin, &info.coin);
    let worker_count_key = info.coin.clone() + ":worker-count:pool";
    let miner_count_key = info.coin.clone() + ":miner-count:pool";

    let ((_, pool_hr), (_, net_hr), (_, minr_cnt), (_, wker_cnt)): (
        (u64, f64),
        (u64, f64),
        (u64, u32),
        (u64, u32),
    ) = match redis::pipe()
        .cmd("ts.get")
        .arg(pool_hashrate_key)
        .cmd("ts.get")
        .arg(net_hashrate_key)
        .cmd("ts.get")
        .arg(miner_count_key)
        .cmd("ts.get")
        .arg(worker_count_key)
        .query_async(&mut con)
        .await
    {
        Ok(r) => r,
        Err(err) => {
            eprintln!("Database error pool overview: {}", err);
            return HttpResponse::InternalServerError().body(
                json!(
                    {"error": "Database error", "result": Value::Null}
                )
                .to_string(),
            );
        }
    };

    HttpResponse::Ok().body(
        json!(
        {"error": Value::Null, "result": {
            "poolHashrate": pool_hr,
            "networkHashrate": net_hr,
            "minerCount": minr_cnt,
            "workerCount": wker_cnt
        }})
        .to_string(),
    )
}

// #[get("/currentHashrate")]
// async fn current_hashrate(
//     info: web::Query<CoinQuery>,
//     api_data: web::Data<SickApiData>,
// ) -> impl Responder {
//     let mut con = api_data.redis.clone();

//     let key = info.coin.clone() + ":hashrate:pool";

//     return ts_get_res::<f64>(&mut con, &key).await;
// }

// #[get("/workerCount")]
// async fn worker_cnt(
//     api_data: web::Data<SickApiData>,
//     info: web::Query<CoinQuery>,
// ) -> impl Responder {
//     let mut con = api_data.redis.clone();
//     let key = info.coin.clone() + ":worker-count:pool";

//     return ts_get_res::<u64>(&mut con, &key).await;
// }

// #[get("/minerCount")]
// async fn miner_cnt(
//     api_data: web::Data<SickApiData>,
//     info: web::Query<CoinQuery>,
// ) -> impl Responder {
//     let mut con = api_data.redis.clone();
//     let key = info.coin.clone() + ":miner-count:pool";

//     return ts_get_res::<u64>(&mut con, &key).await;
// }

async fn ts_get_res<T: serde::Serialize + FromRedisValue>(
    con: &mut ConnectionManager,
    key: &String,
) -> impl Responder {
    let latest: (u64, T) = match con.ts_get(key).await {
        Ok(res) => match res {
            Some(tuple) => tuple,
            None => {
                return HttpResponse::InternalServerError().body(
                    json!(
                        {"error": "Database error", "result": Value::Null}
                    )
                    .to_string(),
                );
            }
        },
        Err(e) => {
            eprintln!("Database error: {}", e);
            return HttpResponse::InternalServerError().body(
                json!(
                    {"error": "Database empty error", "result": Value::Null}
                )
                .to_string(),
            );
        }
    };

    HttpResponse::Ok().body(json!({"result": latest.1, "error": Value::Null}).to_string())
}

#[get("/stakingBalance")]
async fn staking_balance(req: HttpRequest, api_data: web::Data<SickApiData>) -> impl Responder {
    let mut con = api_data.redis.clone();
    // let dbRes : std::result::Result<redis::Value, redis::RedisError> = redis::cmd("TS.GET").arg("VRSC:worker_count")
    //             .query_async(&mut con).await;
    // let dbRes : std::result::Result<u32, redis::RedisError> = redis::cmd("GET").arg("VRSC:1").query_async(&mut con).await;
    let latest_balances: std::result::Result<TsMrange<u64, f64>, redis::RedisError> = con
        .ts_mrange(
            "0",
            "+",
            Some(0),
            None,
            TsFilterOptions::default()
                .with_labels(true)
                .equals("type", "balance"),
        )
        .await;

    match latest_balances {
        Ok(res) => {
            let mut sum: f64 = 0.0;

            for entry in res.values.iter() {
                for values in entry.values.iter() {
                    sum += values.1 * 10.;
                }
            }

            return HttpResponse::Ok().body(sum.to_string());
        }
        Err(err) => {
            eprintln!("err: {}", err);
            return HttpResponse::InternalServerError().body("Data not found");
        }
    }
}

#[get("/blockNumber")]
async fn block_number(req: HttpRequest, api_data: web::Data<SickApiData>) -> impl Responder {
    let mut con = api_data.redis.clone();

    let block_number: std::result::Result<i32, redis::RedisError> =
        con.get("VRSC:block_number").await;

    match block_number {
        Ok(num) => {
            // println!("num {}", num);
            return HttpResponse::Ok().body(num.to_string());
        }
        Err(err) => {
            eprintln!("err: {}", err);
            return HttpResponse::InternalServerError().body("Data not found");
        }
    }
    // return HttpResponse::Ok();
}

#[get("/roundOverview")]
async fn current_effort_pow(
    req: HttpRequest,
    info: web::Query<CoinQuery>,
    api_data: web::Data<SickApiData>,
) -> impl Responder {
    let mut con = api_data.redis.clone();

    let (total, estimated, started): (f64, f64, f64) = match redis::pipe()
        .hget(
            format!("{}:{}:pow:round-effort", &info.coin, &info.coin),
            "$total",
        )
        .hget(
            format!("{}:{}:pow:round-effort", &info.coin, &info.coin),
            "$estimated",
        )
        .hget(
            format!("{}:{}:pow:round-effort", &info.coin, &info.coin),
            "$start",
        )
        .query_async(&mut con)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Database error: {}", e);
            return HttpResponse::InternalServerError().body(
                json!(
                    {"error": "Database error", "result": Value::Null}
                )
                .to_string(),
            );
        }
    };

    let time_now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let elapsed_seconds = (time_now  as f64 - started) / 1000.0;

    let estimated_seconds = estimated / (total / elapsed_seconds);

    HttpResponse::Ok().body(
        json!({
            "error": Value::Null,
            "result": {
                "effort": elapsed_seconds / estimated_seconds,
                "estimatedAt": time_now + estimated_seconds - elapsed_seconds,
                "start": started as u64
            }
        })
        .to_string(),
    )
}

#[get("/blocks")]
async fn blocks(
    info: web::Query<TableQuerySort>,
    api_data: web::Data<SickApiData>,
) -> impl Responder {
    // let offset : u32 = info.page * info.limit as u32;
    //TODO: check validity of sortby and sortdir and chains

    let sort_index = info.coin.clone() + ":block-index:" + &info.sortby;
    // println!("{}", sort_index);

    let mut con = api_data.redis.clone();
    let block_prefix = info.coin.clone() + ":block:";

    let mut cmd = redis::cmd("FCALL"); //make read-only

    cmd.arg("getblocksbyindex")
        .arg(2)
        .arg(sort_index)
        .arg(&block_prefix)
        .arg(info.page * info.limit as u32) // offset
        .arg(info.page * info.limit as u32 + info.limit as u32 - 1); // num (limit)

    if info.sortdir == "desc" {
        cmd.arg("REV");
    }

    return cmd_res::<TableRes<Block>>(&mut con, &cmd).await;
}

#[get("/payouts")]
async fn payouts(info: web::Query<TableQuery>, api_data: web::Data<SickApiData>) -> impl Responder {
    let mut con = api_data.redis.clone();

    let payouts_key = format!("{}:payouts", &info.coin);

    let mut cmd = redis::cmd("FCALL");
    cmd.arg("getpayouts")
        .arg(3)
        .arg(payouts_key)
        .arg(info.page * info.limit as u32) // offset
        .arg(info.page * info.limit as u32 + info.limit as u32 - 1); // num (limit)

    return cmd_res::<TableRes<FinishedPayment>>(&mut con, &cmd).await;
}

#[get("/solvers")]
async fn solvers(
    info: web::Query<TableQuerySort>,
    api_data: web::Data<SickApiData>,
) -> impl Responder {
    let mut con = api_data.redis.clone();
    let solver_index_prefix = info.coin.clone() + ":solver-index:" + &info.sortby.clone();

    let mut cmd = redis::cmd("FCALL"); // make read-only
    cmd.arg("getsolversbyindex")
        .arg(2)
        .arg(solver_index_prefix)
        .arg(info.coin.clone() + ":solver:")
        .arg(info.page * info.limit as u32)
        .arg((info.page + 1) * info.limit as u32 - 1);

    if info.sortdir == "desc" {
        cmd.arg("REV");
    }

    return cmd_res::<TableRes<Solver>>(&mut con, &cmd).await;
}

pub async fn cmd_res<T: serde::Serialize + FromRedisValue>(
    con: &mut ConnectionManager,
    cmd: &redis::Cmd,
) -> impl Responder {
    let res: T = match cmd.query_async(con).await {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Database error: {}", e);
            return HttpResponse::InternalServerError().body(
                json!(
                    {"error": "Database error", "result": Value::Null}
                )
                .to_string(),
            );
        }
    };

    let json_res = json!({"result":
            res, "error": Value::Null
    })
    .to_string();

    return HttpResponse::Ok().body(json_res);
}

impl redis::FromRedisValue for Solver {
    fn from_redis_value(value: &redis::Value) -> redis::RedisResult<Self> {
        let vec: Vec<String> = redis::from_redis_value(value)?;

        if vec.len() != 5 {
            return Err(redis::RedisError::from((
                redis::ErrorKind::TypeError,
                "Failed to parse solver",
                format!("Failed to parse solver {:?}", value),
            )));
        };

        Ok(Solver {
            address: vec[0].clone(),
            hashrate: vec[1].parse().unwrap_or(0f64),
            balance: vec[2].parse().unwrap_or(0f64),
            joined: vec[3].parse().unwrap(), // don't handle
            worker_count: vec[4].parse().unwrap_or(0),
        })
    }
}
