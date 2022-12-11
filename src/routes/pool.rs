use crate::SickApiData;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
extern crate redis;
use std::fmt;

use crate::redis_interop::ffi;
use crate::routes::redis::key_format;

use redis::aio::ConnectionManager;
use redis::{AsyncCommands, FromRedisValue};
use redis_ts::{
    AsyncTsCommands, TsAggregationOptions, TsAggregationType, TsBucketTimestamp, TsFilterOptions,
    TsMrange, TsRange,
}; //TsAggregationOptions, TsBucketTimestamp
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use std::arch::x86_64::_mm_comineq_sd;
use std::collections::HashMap;

use super::block::Block;
use super::payout::FinishedPayment;
use super::solver::{solver_overview, Solver};
use super::table_res::TableRes;
use ffi::Prefix;

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

async fn get_mini_chart(con: &mut ConnectionManager, key: &String) -> HttpResponse {
    let svg_key = key_format(&[&key, "SVG"]);
    let svg_res: Option<String> = match con.get(&svg_key).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Get redis error: {}", e);
            return HttpResponse::InternalServerError().body("Failed to get mini chart");
        }
    };

    let svg_raw: String = match svg_res {
        Some(res) => res,
        None => {
            let points = get_chart_points(con, key).await;

            // if the key doesn't exist, it's time to update chart, regenerate
            if points.len() < 2 {
                return HttpResponse::InternalServerError().body("Not enough data points");
            }

            let mini_chart = make_mini_chart(&points).await;

            let last_interval = (points[points.len() - 1].x - points[points.len() - 2].x) as usize;
            let save_res: String = match con.set_ex(&svg_key, &mini_chart, last_interval).await {
                Ok(e) => e,
                Err(e) => String::from("error"),
            };

            mini_chart
        }
    };

    HttpResponse::Ok()
        .append_header(("Content-Type", "image/svg+xml"))
        .body(svg_raw)
}

// #[derive(Copy)]
struct MiniChart {
    svg: Option<String>,
    interval_ms: u64,
}

pub async fn get_ts_points(con: &mut ConnectionManager, key: &String) -> Vec<(u64, f64)> {
    // Some(
    //         TsAggregationOptions::new(TsAggregationType::Avg(5000))
    //             .empty(true)
    //             .bucket_timestamp(Some(TsBucketTimestamp::Mid))
    //    )
    let tms: TsRange<u64, f64> = match con
        .ts_range(key, 0, "+", None::<usize>, None::<TsAggregationOptions>)
        .await
    {
        Ok(res) => res,
        Err(err) => {
            eprintln!("range query error: {}", err);
            return Vec::<(u64, f64)>::new();
        }
    };
    tms.values
}

pub async fn get_chart_points(
    con: &mut ConnectionManager,
    key: &String,
) -> Vec<chartgen::Point<f64>> {
    let raw_points = get_ts_points(con, key).await;
    let mut res: Vec<chartgen::Point<f64>> = Vec::new();
    res.reserve(raw_points.len());

    for i in raw_points.iter() {
        res.push(chartgen::Point {
            x: i.0 as f64,
            y: i.1,
        });
    }

    res
}

pub async fn make_mini_chart(points: &Vec<chartgen::Point<f64>>) -> String {
    let chart_size: chartgen::Point<f64> = chartgen::Point { x: 150.0, y: 100.0 };
    let stroke_width = String::from("1.5");
    let color = String::from("red");

    let truncated = chartgen::truncate_chart(&points, chart_size);

    chartgen::generate_svg(&truncated, chart_size, color.clone(), stroke_width.clone())
}

pub fn pool_route(cfg: &mut web::ServiceConfig) {
    cfg.service(pool_overview);
    cfg.service(hashrate_history);
    cfg.service(staking_balance);
    cfg.service(block_number);
    cfg.service(current_effort_pow);
    cfg.service(blocks);
    cfg.service(payouts);
    cfg.service(solvers);
    cfg.service(charts_hashrate_history);
    cfg.service(payout_overview);
}

// #[get("/charts/hashrateHistory.svg")]
#[get("/charts/{chart_name}.svg")]
async fn charts_hashrate_history(
    api_data: web::Data<SickApiData>,
    info: web::Query<CoinQuery>,
    path: web::Path<String>,
) -> impl Responder {
    let mut con = api_data.redis.clone();
    let chart_name = path.into_inner();
    let coin = info.coin.clone();

    let map: HashMap<&str, String> = HashMap::from([
        (
            "hashrateHistory",
            key_format(&[
                &info.coin,
                &Prefix::HASHRATE.to_string(),
                &Prefix::POOL.to_string(),
                &Prefix::COMPACT.to_string(),
            ]),
        ),
        (
            "networkHashrateHistory",
            key_format(&[
                &info.coin,
                &Prefix::HASHRATE.to_string(),
                &Prefix::NETWORK.to_string(),
                &Prefix::COMPACT.to_string(),
            ]),
        ),
        (
            "minerCountHistory",
            key_format(&[
                &info.coin,
                &Prefix::MINER_COUNT.to_string(),
                &Prefix::POOL.to_string(),
                &Prefix::COMPACT.to_string(),
            ]),
        ),
        (
            "workerCountHistory",
            key_format(&[
                &info.coin,
                &Prefix::WORKER_COUNT.to_string(),
                &Prefix::POOL.to_string(),
                &Prefix::COMPACT.to_string(),
            ]),
        ),
        (
            "difficultyHistory",
            key_format(&[
                &info.coin,
                &Prefix::DIFFICULTY.to_string(),
                &Prefix::COMPACT.to_string(),
            ]),
        ),
        (
            "blockCountHistory",
            key_format(&[
                &info.coin,
                &Prefix::MINED_BLOCK.to_string(),
                &Prefix::COMPACT.to_string(),
            ]),
        ),
        (
            "blockEffortHistory",
            key_format(&[
                &info.coin,
                &Prefix::BLOCK.to_string(),
                &Prefix::COMPACT.to_string(),
            ]),
        ),
    ]);

    match map.get(&chart_name[..]) {
        Some(key) => get_mini_chart(&mut con, &key).await,
        None => HttpResponse::NotFound().body("no such chart"),
    }
}

#[get("/{key}History")]
async fn hashrate_history(
    api_data: web::Data<SickApiData>,
    info: web::Query<CoinQuery>,
    path: web::Path<String>,
) -> impl Responder {
    let mut con = api_data.redis.clone();

    let key_prefix = path.into_inner();

    let map: HashMap<&str, String> = HashMap::from([
        (
            "hashrate",
            key_format(&[
                &info.coin,
                &Prefix::HASHRATE.to_string(),
                &Prefix::POOL.to_string(),
            ]),
        ),
        (
            "networkHashrate",
            key_format(&[
                &info.coin,
                &Prefix::HASHRATE.to_string(),
                &Prefix::NETWORK.to_string(),
            ]),
        ),
        (
            "minerCount",
            key_format(&[
                &info.coin,
                &Prefix::MINER_COUNT.to_string(),
                &Prefix::POOL.to_string(),
            ]),
        ),
        (
            "workerCount",
            key_format(&[
                &info.coin,
                &Prefix::WORKER_COUNT.to_string(),
                &Prefix::POOL.to_string(),
            ]),
        ),
        (
            "difficulty",
            key_format(&[&info.coin, &Prefix::DIFFICULTY.to_string()]),
        ),
        (
            "blockCount",
            key_format(&[&info.coin, &Prefix::MINED_BLOCK.to_string()]),
        ),
        (
            "blockEffort",
            key_format(&[&info.coin, &Prefix::BLOCK.to_string()]),
        ),
    ]);

    let key: &String;
    match map.get(&key_prefix[..]) {
        Some(index) => {
            key = index;
        }
        None => {
            return HttpResponse::NotFound().body("no such chart");
        }
    };

    history(&mut con, key).await
}

async fn history(con: &mut ConnectionManager, key: &String) -> HttpResponse {
    let mut points = get_ts_points(con, key).await;

    for el in points.iter_mut() {
        // time in unix seconds not ms to save bandwidth
        el.0 /= 1000;
    }

    HttpResponse::Ok().body(
        json!({
            "error": Value::Null,
            "result": points
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

    let pool_hashrate_key = format!("{}:HASHRATE:POOL", &info.coin);
    // add per coin net hashrate
    let net_hashrate_key = format!("{}:HASHRATE:NETWORK", &info.coin);
    let worker_count_key = info.coin.clone() + ":WORKER_COUNT:POOL";
    let miner_count_key = info.coin.clone() + ":MINER_COUNT:POOL";

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
        {
            "error": Value::Null,
            "result": {
                "poolHashrate": pool_hr,
                "networkHashrate": net_hr,
                "minerCount": minr_cnt,
                "workerCount": wker_cnt
            }
        })
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
            None::<TsAggregationType>,
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

    let (total, estimated, started): (f64, f64, f64) = match redis::cmd("HMGET")
        .arg(key_format(&[
            &info.coin,
            &Prefix::ROUND.to_string(),
            &Prefix::EFFORT.to_string(),
        ]))
        .arg(Prefix::TOTAL_EFFORT.to_string())
        .arg(Prefix::ESTIMATED_EFFORT.to_string())
        .arg(Prefix::START_TIME.to_string())
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

    HttpResponse::Ok().body(
        json!({
            "error": Value::Null,
            "result": {
                "effortPercent": total / estimated * 100.0,
                // "estimatedAt": time_now as f64 + (estimated_seconds - elapsed_seconds),
                "start": started as u64
            }
        })
        .to_string(),
    )
}

impl fmt::Display for ffi::Prefix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[get("/blocks")]
async fn blocks(
    info: web::Query<TableQuerySort>,
    api_data: web::Data<SickApiData>,
) -> impl Responder {
    // let offset : u32 = info.page * info.limit as u32;
    //TODO: check validity of sortby and sortdir and chains
    use ffi::Prefix;

    let mut con = api_data.redis.clone();

    let block_prefix = key_format(&[&info.coin, &Prefix::BLOCK.to_string()]);
    let sort_index = key_format(&[
        &block_prefix,
        &Prefix::INDEX.to_string(),
        &info.sortby.to_uppercase(),
    ]);

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

    let mut table_res: TableRes<Block> = cmd.query_async(&mut con).await.`unwrap();
    let mut addresses_pipe = redis::pipe();
    for i in table_res.entries.iter_mut() {
        let key = key_format(&[
            &info.coin,
            &Prefix::SOLVER.to_string(),
            &format!("{:08x}", i.raw.miner_id),
        ]);
        addresses_pipe
            .cmd("HGET")
            .arg(key)
            .arg(Prefix::ADDRESS.to_string());
    }
    let addresses: Vec<String> = match addresses_pipe.query_async(&mut con).await {
        Ok(r) => r,
        Err(e) => {
            return redis_error();
        }
    };
    let mut addr_iter = addresses.iter();
    for b in table_res.entries.iter_mut() {
        b.solver = addr_iter.next().unwrap().clone();
    }

    return HttpResponse::Ok().body(json!({"result": table_res, "error": Value::Null}).to_string());
}

fn redis_error() -> HttpResponse {
    HttpResponse::InternalServerError().body(
        json!(
            {"error": "Database error", "result": Value::Null}
        )
        .to_string(),
    )
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
    let solver_index_prefix =
        info.coin.clone() + ":SOLVER:INDEX:" + &info.sortby.clone().to_uppercase();

    let mut cmd = redis::cmd("FCALL"); // make read-only
    cmd.arg("getsolversbyindex")
        .arg(2)
        .arg(solver_index_prefix)
        .arg(info.coin.clone() + ":SOLVER:")
        .arg(info.page * info.limit as u32)
        .arg((info.page + 1) * info.limit as u32 - 1);

    if info.sortdir == "desc" {
        cmd.arg("REV");
    }

    return cmd_res::<TableRes<Solver>>(&mut con, &cmd).await;
}

#[get("/payoutOverview")]
async fn payout_overview(
    info: web::Query<CoinQuery>,
    api_data: web::Data<SickApiData>,
) -> impl Responder {
    // let mut con = api_data.redis.clone();
    
    HttpResponse::Ok().body(json!({
        "error": Value::Null,
        "scheme": "PPLNS",
        "fee": 0.01,
        "next_payout": 12340000,
        "minimum_threshold": 1,
        "total_paid": 1,
        "total_payouts": 1,
    }).to_string())
}

pub async fn cmd_res<T: serde::Serialize + FromRedisValue>(
    con: &mut ConnectionManager,
    cmd: &redis::Cmd,
) -> impl Responder {
    let res: T = match cmd.query_async(con).await {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Database error: {}", e);
            return redis_error();
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

// round effort and blocks mined 1 day interval, 30d capacity
