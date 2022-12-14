use crate::SickApiData;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
extern crate redis;
use std::fmt;

use crate::redis_interop::ffi::{self, BlockSubmission};
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

use super::payout::FinishedPayment;
use super::solver::{solver_overview, Solver};
use super::table_res::TableRes;
use ffi::Prefix;
use mysql::prelude::*;
use mysql::*;

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

    let (total, estimated, started): (f64, f64, f64) = match redis::cmd("ZSCORE")
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
    let mut con = api_data.mysql.get_conn().unwrap();

    if ((&info.sortdir != "asc" && &info.sortdir != "desc")
        || (info.sortdir != "id"
            && info.sortdir != "reward"
            && info.sortdir != "duration_ms"
            && info.sortdir != "difficulty"
            && info.sortdir != "effort_percent"))
    {
        HttpResponse::BadRequest().body("invalid sort");
    }

    let query =
     format!("SELECT blocks.id,status,hash,reward,time_ms,duration_ms,height,difficulty,effort_percent,addresses.address FROM blocks INNER JOIN addresses ON addresses.id = blocks.miner_id ORDER BY {} {} LIMIT {},{}", &info.sortby, &info.sortdir, info.page * info.limit as u32, info.limit);

    let total: i64 = con
        .query_first("SELECT COUNT(*) FROM blocks")
        .unwrap()
        .unwrap();

    let blocks = con
        .query_map(
            query,
            |(
                id,
                status,
                hash_hex,
                reward,
                time_ms,
                duration_ms,
                height,
                difficulty,
                effort_percent,
                solver,
            ): (u32, u8, String, u64, u64, u64, u32, f64, f64, String)| {
                BlockSubmission {
                    id,
                    confirmations: 0,
                    block_type: 0,
                    chain: 0,
                    reward,
                    time_ms,
                    duration_ms,
                    height,
                    difficulty,
                    effort_percent,
                    hash_hex,
                    solver,
                }
            },
        )
        .unwrap();

    let res = TableRes::<BlockSubmission> {
        total,
        entries: blocks,
    };

    return HttpResponse::Ok().body(json!({"result": res, "error": Value::Null}).to_string());
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
    let mut con_redis = api_data.redis.clone();
    let mut con_mysql = api_data.mysql.get_conn().unwrap();

    let solver_index_prefix =
        info.coin.clone() + ":SOLVER:INDEX:" + &info.sortby.clone().to_uppercase();

    let round_effort_key = key_format(&[
        &info.coin,
        &Prefix::ROUND.to_string(),
        &Prefix::EFFORT.to_string(),
    ]);

    let hashrate_index = key_format(&[
        &info.coin,
        &Prefix::SOLVER.to_string(),
        &Prefix::INDEX.to_string(),
        &Prefix::HASHRATE.to_string(),
    ]);

    let mut ids: Option<Vec<u32>> = None;

    if info.sortby == "round-effort" {
        // miner id -> effort
        ids = zrange_table(&mut con_redis, &round_effort_key, &info).await;
    } else if info.sortby == "hashrate" {
        ids = zrange_table(&mut con_redis, &hashrate_index, &info).await;
    }

    let ids = match ids {
        Some(r) => r,
        None => {
            return redis_error();
        }
    };

    let round_info_key = key_format(&[&info.coin, &Prefix::ROUND.to_string(), &Prefix::INFO.to_string()]);
    let total_effort : f64 = con_redis.hget(round_info_key, Prefix::TOTAL_EFFORT.to_string()).await.unwrap();

    // HANDLE
    let hashrates: Vec<Option<f64>> = zscore_ids(&mut con_redis, &hashrate_index, &ids)
        .await
        .unwrap();
    let efforts: Vec<Option<f64>> = zscore_ids(&mut con_redis, &round_effort_key, &ids)
        .await
        .unwrap();

    let mut miners: Vec<Solver> = Vec::new();
    miners.reserve(ids.len());
    
    let stmt = con_mysql.prep("SELECT addresses.address,join_time FROM miners INNER JOIN addresses ON miners.address_id = addresses.id WHERE address_id = ?").unwrap();
    
    for (i, id) in ids.iter().enumerate() {
        let (addr, join_time) : (String, u64) = con_mysql.exec_first(&stmt, (id,)).unwrap().unwrap();
        
        miners.push(Solver {
            id: *id,
            address: addr,
            hashrate: match hashrates[i] {
                Some(r) => r,
                None => 0.0,
            },
            round_effort: match efforts[i] {
                Some(r) => r / total_effort,
                None => 0.0,
            },
            balance: 0,
            joined: join_time,
        });
    }


    let res = TableRes::<Solver> {
        total: 0,
        entries: miners,
    };

    return HttpResponse::Ok().body(json!({"result": res, "error": Value::Null}).to_string());
}

async fn zrange_table(
    con: &mut ConnectionManager,
    key: &String,
    query: &web::Query<TableQuerySort>,
) -> Option<Vec<u32>> {
    match redis::cmd("ZRANGE")
        .arg(key)
        .arg(query.page * query.limit as u32)
        .arg((query.page + 1) * query.limit as u32)
        .query_async(con)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}", e);
            None
        }
    }
}

async fn zscore_ids(
    con: &mut ConnectionManager,
    key: &String,
    ids: &Vec<u32>,
) -> Option<Vec<Option<f64>>> {
    let mut pipe = redis::pipe();
    for id in ids.iter() {
        pipe.cmd("ZSCORE").arg(key).arg(id);
    }

    let res: Option<Vec<Option<f64>>> = match pipe.query_async(con).await {
        Ok(r) => r,
        Err(e) => None,
    };
    res
}

#[get("/payoutOverview")]
async fn payout_overview(
    info: web::Query<CoinQuery>,
    api_data: web::Data<SickApiData>,
) -> impl Responder {
    // let mut con = api_data.redis.clone();

    HttpResponse::Ok().body(
        json!({
            "error": Value::Null,
            "scheme": "PPLNS",
            "fee": 0.01,
            "next_payout": 12340000,
            "minimum_threshold": 1,
            "total_paid": 1,
            "total_payouts": 1,
        })
        .to_string(),
    )
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

// impl redis::FromRedisValue for Solver {
//     fn from_redis_value(value: &redis::Value) -> redis::RedisResult<Self> {
//         let vec: Vec<String> = redis::from_redis_value(value)?;

//         if vec.len() != 5 {
//             return Err(redis::RedisError::from((
//                 redis::ErrorKind::TypeError,
//                 "Failed to parse solver",
//                 format!("Failed to parse solver {:?}", value),
//             )));
//         };

//         Ok(Solver {
//             address: vec[0].clone(),
//             hashrate: vec[1].parse().unwrap_or(0f64),
//             balance: vec[2].parse().unwrap_or(0f64),
//             joined: vec[3].parse().unwrap(), // don't handle
//             // worker_count: vec[4].parse().unwrap_or(0),
//         })
//     }
// }

// round effort and blocks mined 1 day interval, 30d capacity
