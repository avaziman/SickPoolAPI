use crate::api_data::{CoinQuery, SickApiData};
use crate::routes::solver::SolverInfo;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
extern crate redis;
use std::fmt;

use super::block::BlockSubmission;
use crate::redis_interop::ffi::{self};
use crate::routes::redis::{get_ts_points, key_format};

use redis::aio::ConnectionManager;
use redis::{AsyncCommands, FromRedisValue};
use redis_ts::{
    AsyncTsCommands, TsAggregationType, TsBucketTimestamp, TsFilterOptions, TsMrange, TsRange,
};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use std::collections::HashMap;

use super::history::{self, TimeSeriesInterval};
use super::payout::Payout;
use super::solver::{solver_overview, Solver};
use super::table_res::TableRes;
use ffi::Prefix;
use mysql::prelude::*;
use mysql::*;

// use redisearch_api::{init, Document, FieldType, Index, TagOptions};
use serde::Deserialize;

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
    cfg.service(block_overview);
    cfg.service(blocks);
    cfg.service(payouts);
    cfg.service(miners);
    // cfg.service(charts_hashrate_history);
    cfg.service(payout_overview);
    cfg.service(web::scope("/history").configure(history::pool_history_route));
}

// async fn get_mini_chart(con: &mut ConnectionManager, key: &String) -> HttpResponse {
//     let svg_key = key_format(&[&key, "SVG"]);
//     let svg_res: Option<String> = match con.get(&svg_key).await {
//         Ok(r) => r,
//         Err(e) => {
//             eprintln!("Get redis error: {}", e);
//             return HttpResponse::InternalServerError().body("Failed to get mini chart");
//         }
//     };

//     let svg_raw: String = match svg_res {
//         Some(res) => res,
//         None => {
//             let points = get_chart_points(con, key).await;

//             // if the key doesn't exist, it's time to update chart, regenerate
//             if points.len() < 2 {
//                 return HttpResponse::InternalServerError().body("Not enough data points");
//             }

//             let mini_chart = make_mini_chart(&points).await;

//             let last_interval = (points[points.len() - 1].x - points[points.len() - 2].x) as usize;
//             let save_res: String = match con.set_ex(&svg_key, &mini_chart, last_interval).await {
//                 Ok(e) => e,
//                 Err(e) => String::from("error"),
//             };

//             mini_chart
//         }
//     };

//     HttpResponse::Ok()
//         .append_header(("Content-Type", "image/svg+xml"))
//         .body(svg_raw)
// }

// #[derive(Copy)]
struct MiniChart {
    svg: Option<String>,
    interval_ms: u64,
}

// pub async fn get_chart_points(
//     con: &mut ConnectionManager,
//     key: &String,
//     interval: &TimeSeriesInterval,
// ) -> Vec<chartgen::Point<f64>> {
//     let raw_points = get_ts_points(con, key, interval).await;
//     let mut res: Vec<chartgen::Point<f64>> = Vec::new();
//     res.reserve(raw_points.len());

//     for i in raw_points.iter() {
//         res.push(chartgen::Point {
//             x: i.0 as f64,
//             y: i.1,
//         });
//     }

//     res
// }

// pub async fn make_mini_chart(points: &Vec<chartgen::Point<f64>>) -> String {
//     let chart_size: chartgen::Point<f64> = chartgen::Point { x: 150.0, y: 100.0 };
//     let stroke_width = String::from("1.5");
//     let color = String::from("red");

//     let truncated = chartgen::truncate_chart(&points, chart_size);

//     chartgen::generate_svg(&truncated, chart_size, color.clone(), stroke_width.clone())
// }

// #[get("/charts/hashrateHistory.svg")]
// #[get("/charts/{chart_name}.svg")]
// async fn charts_hashrate_history(
//     api_data: web::Data<SickApiData>,
//     info: web::Query<CoinQuery>,
//     path: web::Path<String>,
// ) -> impl Responder {
//     let mut con = api_data.redis.clone();
//     let chart_name = path.into_inner();
//     let coin = info.coin.clone();

//     let map: HashMap<&str, String> = HashMap::from([
//         (
//             "hashrateHistory",
//             key_format(&[
//                 &info.coin,
//                 &Prefix::HASHRATE.to_string(),
//                 &Prefix::POOL.to_string(),
//                 &Prefix::COMPACT.to_string(),
//             ]),
//         ),
//         (
//             "networkHashrateHistory",
//             key_format(&[
//                 &info.coin,
//                 &Prefix::HASHRATE.to_string(),
//                 &Prefix::NETWORK.to_string(),
//                 &Prefix::COMPACT.to_string(),
//             ]),
//         ),
//         (
//             "minerCountHistory",
//             key_format(&[
//                 &info.coin,
//                 &Prefix::MINER_COUNT.to_string(),
//                 &Prefix::POOL.to_string(),
//                 &Prefix::COMPACT.to_string(),
//             ]),
//         ),
//         (
//             "workerCountHistory",
//             key_format(&[
//                 &info.coin,
//                 &Prefix::WORKER_COUNT.to_string(),
//                 &Prefix::POOL.to_string(),
//                 &Prefix::COMPACT.to_string(),
//             ]),
//         ),
//         (
//             "difficultyHistory",
//             key_format(&[
//                 &info.coin,
//                 &Prefix::DIFFICULTY.to_string(),
//                 &Prefix::COMPACT.to_string(),
//             ]),
//         ),
//         (
//             "blockCountHistory",
//             key_format(&[
//                 &info.coin,
//                 &Prefix::MINED_BLOCK.to_string(),
//                 &Prefix::COMPACT.to_string(),
//             ]),
//         ),
//         (
//             "blockEffortHistory",
//             key_format(&[
//                 &info.coin,
//                 &Prefix::BLOCK.to_string(),
//                 &Prefix::COMPACT.to_string(),
//             ]),
//         ),
//     ]);

//     match map.get(&chart_name[..]) {
//         Some(key) => get_mini_chart(&mut con, &key).await,
//         None => HttpResponse::NotFound().body("no such chart"),
//     }
// }

// #[get("/history/{key}")]
// async fn hashrate_history(
//     api_data: web::Data<SickApiData>,
//     info: web::Query<CoinQuery>,
//     path: web::Path<String>,
// ) -> impl Responder {
//     let mut con = api_data.redis.clone();

//     let key_prefix = path.into_inner();

//     let map: HashMap<&str, String> = HashMap::from([
//         (
//             "hashrate",
//             key_format(&[
//                 &info.coin,
//                 &Prefix::HASHRATE.to_string(),
//                 &Prefix::POOL.to_string(),
//             ]),
//         ),
//         (
//             "networkHashrate",
//             key_format(&[
//                 &info.coin,
//                 &Prefix::HASHRATE.to_string(),
//                 &Prefix::NETWORK.to_string(),
//             ]),
//         ),
//         (
//             "minerCount",
//             key_format(&[
//                 &info.coin,
//                 &Prefix::MINER_COUNT.to_string(),
//                 &Prefix::POOL.to_string(),
//             ]),
//         ),
//         (
//             "workerCount",
//             key_format(&[
//                 &info.coin,
//                 &Prefix::WORKER_COUNT.to_string(),
//                 &Prefix::POOL.to_string(),
//             ]),
//         ),
//         (
//             "difficulty",
//             key_format(&[&info.coin, &Prefix::DIFFICULTY.to_string()]),
//         ),
//         (
//             "blockCount",
//             key_format(&[&info.coin, &Prefix::MINED_BLOCK.to_string()]),
//         ),
//         (
//             "blockEffort",
//             key_format(&[&info.coin, &Prefix::BLOCK.to_string()]),
//         ),
//         (
//             "blocksMined",
//             key_format(&[&info.coin, &Prefix::MINED_BLOCK.to_string(), &Prefix::NUMBER.to_string()]),
//         ),
//     ]);

//     let key: &String;
//     match map.get(&key_prefix[..]) {
//         Some(index) => {
//             key = index;
//         }
//         None => {
//             return HttpResponse::NotFound().body("no such chart");
//         }
//     };
// }

#[get("/overview")]
async fn pool_overview(
    info: web::Query<CoinQuery>,
    api_data: web::Data<SickApiData>,
) -> impl Responder {
    let mut con = api_data.redis.clone();

    let pool_hashrate_key = format!("{}:HASHRATE:POOL", &info.coin);
    // add per coin net hashrate
    let net_hashrate_key = format!("{}:HASHRATE:NETWORK", &info.coin);
    let miner_count_key = info.coin.clone() + ":MINER_COUNT:POOL";
    let worker_count_key = info.coin.clone() + ":WORKER_COUNT:POOL";

    let ((_, pool_hr), (_, net_hr), (_, minr_cnt), (_, wker_cnt)): (
        (u64, f64),
        (u64, f64),
        (u64, u32),
        (u64, u32),
    ) = match redis::pipe()
        .cmd("TS.GET")
        .arg(pool_hashrate_key)
        .cmd("TS.GET")
        .arg(net_hashrate_key)
        .cmd("TS.GET")
        .arg(miner_count_key)
        .cmd("TS.GET")
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

#[get("/blockOverview")]
async fn block_overview(
    req: HttpRequest,
    info: web::Query<CoinQuery>,
    api_data: web::Data<SickApiData>,
) -> impl Responder {
    let mut con = api_data.redis.clone();
    let mut mysql_con = api_data.mysql.get_conn().unwrap();

    let (total, estimated, started): (f64, f64, f64) = match redis::cmd("HMGET")
        .arg(key_format(&[
            &info.coin,
            &Prefix::ROUND.to_string(),
            &Prefix::STATS.to_string(),
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
            return redis_error();
        }
    };

    let height: u32 = match con
        .hget(
            key_format(&[
                &info.coin,
                &Prefix::BLOCK.to_string(),
                &Prefix::STATS.to_string(),
            ]),
            key_format(&[&Prefix::HEIGHT.to_string()]),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return redis_error();
        }
    };

    let curtime = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mined_key = key_format(&[
        &info.coin,
        &Prefix::MINED_BLOCK.to_string(),
        &Prefix::NUMBER.to_string(),
    ]);

    let mined24h_ts: TsRange<u64, u64> = match con
        .ts_range(
            mined_key,
            (curtime - api_data.block_interval.interval) * 1000,
            curtime * 1000,
            Some(1),
            Some(TsAggregationType::Sum(
                api_data.block_interval.interval * 1000,
            )),
        )
        .await
    {
        Ok(o) => o,
        Err(e) => {
            return redis_error();
        }
    };

    let mined24h = if mined24h_ts.values.len() > 0 {
        mined24h_ts.values[0].1
    } else {
        0
    };

    let key = key_format(&[&info.coin, &Prefix::DIFFICULTY.to_string()]);
    let difficulty: Option<(u64, f64)> = con.ts_get(key).await.unwrap();
    let difficulty = difficulty.unwrap_or((0, 0.0));

    let (mined, orphaned, average_effort, average_duration): (u64, u64, f64, f64) = mysql_con
        .query_first(
            "SELECT mined, orphaned, COALESCE(0, effort_percent / mined), COALESCE(0, duration_ms / mined) FROM block_stats",
        )
        .unwrap()
        .unwrap();

    HttpResponse::Ok().body(
        json!({
            "error": Value::Null,
            "result": {
                "currentRound": {
                    "effortPercent": total / estimated * 100.0,
                    "startTime": started as u64
                },
                "height": height,
                "orphans": orphaned,
                "mined": mined,
                "averageEffort": average_effort,
                "averageDuration": average_duration,
                "mined24h": mined24h,
                "difficulty": difficulty,
                "confirmations": 10
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
    mut info: web::Query<TableQuerySort>,
    api_data: web::Data<SickApiData>,
) -> impl Responder {
    let mut con = api_data.mysql.get_conn().unwrap();
    info.sortdir = get_sortdir(&info.sortdir);

    if info.sortby != "id"
        && info.sortby != "status"
        && info.sortby != "reward"
        && info.sortby != "duration_ms"
        && info.sortby != "difficulty"
        && info.sortby != "effort_percent"
    {
        HttpResponse::BadRequest().body("invalid sort");
    }

    let query =
     format!("SELECT blocks.id,status,hash,reward,time_ms,duration_ms,height,difficulty,effort_percent,addresses.address_md5 FROM blocks INNER JOIN addresses ON addresses.id = blocks.miner_id ORDER BY {} {} LIMIT {},{}", &info.sortby, &info.sortdir, info.page * info.limit as u32, info.limit);

    let total: u64 = con
        .query_first("SELECT mined FROM block_stats")
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
            ): (u32, i8, String, u64, u64, u64, u32, f64, f64, String)| {
                BlockSubmission {
                    id,
                    status,
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

pub fn redis_error() -> HttpResponse {
    HttpResponse::InternalServerError().body(
        json!(
            {"error": "Database error", "result": Value::Null}
        )
        .to_string(),
    )
}

#[get("/payouts")]
async fn payouts(info: web::Query<TableQuery>, api_data: web::Data<SickApiData>) -> impl Responder {
    let mut con = api_data.mysql.get_conn().unwrap();

    let payouts_vec = con
        .query_map(
            "SELECT id, txid, payee_amount, paid_amount, tx_fee, time_ms FROM payouts",
            |(id, txid, payee_amount, paid_amount, tx_fee, time_ms): (
                u32,
                String,
                u32,
                u64,
                u64,
                u64,
            )| {
                Payout {
                    id,
                    txid,
                    payee_amount,
                    paid_amount,
                    tx_fee,
                    time_ms,
                }
            },
        )
        .unwrap();

    let total: u64 = con
        .query_first("SELECT count FROM payout_stats")
        .unwrap()
        .unwrap();

    let res = TableRes::<Payout> {
        total: total,
        entries: payouts_vec,
    };

    return HttpResponse::Ok().body(json!({"result": res, "error": Value::Null}).to_string());
}

fn get_sortdir(s: &String) -> String {
    if s.to_lowercase() == "asc" {
        return String::from("ASC");
    }
    return String::from("DESC");
}

#[get("/miners")]
async fn miners(
    mut info: web::Query<TableQuerySort>,
    api_data: web::Data<SickApiData>,
) -> impl Responder {
    let mut con_redis = api_data.redis.clone();
    let mut con_mysql = api_data.mysql.get_conn().unwrap();

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

    let mut ids: Option<Vec<u32>> = Some(Vec::new());
    info.sortdir = get_sortdir(&info.sortdir);

    let redis_index = if info.sortby == "round-effort" {
        // miner id -> effort
        ids = zrange_table(&mut con_redis, &round_effort_key, &info).await;
        true
    } else if info.sortby == "hashrate" {
        ids = zrange_table(&mut con_redis, &hashrate_index, &info).await;
        true
    } else if info.sortby == "mature_balance" || info.sortby == "join_time" {
        false
    } else {
        return redis_error();
    };

    let mut ids = match ids {
        Some(r) => r,
        None => {
            return redis_error();
        }
    };

    let mut miners_info: Vec<SolverInfo> = Vec::new();
    let mut miners: Vec<Solver> = Vec::new();

    if redis_index {
        let stmt = con_mysql.prep("SELECT addresses.address_md5,join_time,mature_balance FROM miners INNER JOIN addresses ON miners.address_id = addresses.id WHERE address_id = ?").unwrap();
        if miners.len() < info.limit as usize {
            let remaining = info.limit as usize - miners.len();
        }

        for id in ids.iter() {
            let (address, joined, mature_balance): (String, u64, u64) =
                match con_mysql.exec_first(&stmt, (id,)) {
                    Ok(r) => match r {
                        Some(r) => r,
                        None => continue,
                    },
                    Err(e) => {
                        println!("Miner with id {} not found in mysql db", id);
                        continue;
                    }
                };
            miners_info.push(SolverInfo {
                address,
                joined,
                mature_balance,
            });
        }

    } else {
        miners_info = match con_mysql.query_map(format!("SELECT id,addresses.address_md5,join_time,mature_balance FROM miners INNER JOIN addresses ON miners.address_id = addresses.id ORDERBY {} {} LIMIT {}", &info.sortby, &info.sortdir, &info.limit), |(id, address, joined, mature_balance): (u32, String, u64, u64)|{
            ids.push(id);
            SolverInfo {
                address,
                joined,
                mature_balance
            }
        }){
            Ok(r) => r,
            Err(r) => {return redis_error();}
        };
    }

    let round_info_key = key_format(&[
        &info.coin,
        &Prefix::ROUND.to_string(),
        &Prefix::STATS.to_string(),
    ]);
    let total_effort: f64 = match con_redis
        .hget(round_info_key, Prefix::TOTAL_EFFORT.to_string())
        .await
    {
        Ok(r) => r,
        Err(err) => 0.0,
    };

    // HANDLE
    let hashrates: Vec<Option<f64>> = zscore_ids(&mut con_redis, &hashrate_index, &ids)
        .await
        .unwrap();
    let efforts: Vec<Option<f64>> = zscore_ids(&mut con_redis, &round_effort_key, &ids)
        .await
        .unwrap();

    miners.reserve(ids.len());

    for (i, info) in miners_info.iter().enumerate(){
            miners.push(Solver {
                id: ids[i],
                hashrate: match hashrates[i] {
                    Some(r) => r,
                    None => 0.0,
                },
                round_effort: match efforts[i] {
                    Some(r) => r / total_effort,
                    None => 0.0,
                },
                info: info.clone()
            });

    }

    let total: u64 = con_mysql
        .query_first("SELECT COUNT(*) FROM miners")
        .unwrap()
        .unwrap();

    let res = TableRes::<Solver> {
        total: total,
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
    let mut mysql_con = api_data.mysql.get_conn().unwrap();

    let curtime = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let (count, amount, next): (u64, u64, i64) = mysql_con
        .query_first("SELECT count, amount, next_ms FROM payout_stats")
        .unwrap()
        .unwrap();

    let next: i64 = if curtime > next { -1 } else { next };

    HttpResponse::Ok().body(
        json!({
            "error": Value::Null,
            "nextPayout": next,
            "totalPaid": amount,
            "totalPayouts": count,
            "scheme": "PPLNS",
            "fee": api_data.fee,
            "minimumThreshold": api_data.min_payout,
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
