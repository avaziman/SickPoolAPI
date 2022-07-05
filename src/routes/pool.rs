use crate::SickApiData;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
extern crate redis;

use redis::AsyncCommands;
use redis_ts::{AsyncTsCommands, TsAggregationType, TsFilterOptions, TsMget, TsMrange, TsOptions, TsRange};
use serde_json::{json, Value};
use std::collections::HashMap;

use super::block::Block;
use super::solver::Solver;
use super::table_res::TableRes;

// use redisearch_api::{init, Document, FieldType, Index, TagOptions};
use serde::Deserialize;

// #[get("/currentHashrate")]
// async fn current_hashrate(req: HttpRequest, api_data: web::Data<SickApiData>) -> impl Responder {
//     let mut con = api_data.redis.clone();

//     let latest: (u64, f64) = match con.ts_get("pool_hashrate").await {
//         Ok(res) => match res {
//             Some(tuple) => tuple,
//             None => {
//                 return HttpResponse::InternalServerError().body(
//                     json!(
//                         {"error": "Database error", "result": Value::Null}
//                     )
//                     .to_string(),
//                 );
//             }
//         },
//         Err(e) => {
//             eprintln!("Database error: {}", e);
//             return HttpResponse::InternalServerError().body(
//                 json!(
//                     {"error": "Database error", "result": Value::Null}
//                 )
//                 .to_string(),
//             );
//         }
//     };

//     HttpResponse::Ok().body(json!({"result": latest.1, "error": Value::Null}).to_string())
// }

#[get("/workerCount")]
async fn worker_count(req: HttpRequest, api_data: web::Data<SickApiData>) -> impl Responder {
    let mut con = api_data.redis.clone();
    // let dbRes : std::result::Result<redis::Value, redis::RedisError> = redis::cmd("TS.GET").arg("VRSC:worker_count")
    //             .query_async(&mut con).await;
    // let dbRes : std::result::Result<u32, redis::RedisError> = redis::cmd("GET").arg("VRSC:1").query_async(&mut con).await;
    let latest: (u64, f64) = match con.ts_get("my_engine").await {
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
                    {"error": "Database error", "result": Value::Null}
                )
                .to_string(),
            );
        }
    };

    HttpResponse::Ok().body(json!({"result": latest.1, "error": Value::Null}).to_string())
}

// #[get("/stakingBalance")]
// async fn staking_balance(req: HttpRequest, api_data: web::Data<SickApiData>) -> impl Responder {
//     let mut con = api_data.redis.clone();
//     // let dbRes : std::result::Result<redis::Value, redis::RedisError> = redis::cmd("TS.GET").arg("VRSC:worker_count")
//     //             .query_async(&mut con).await;
//     // let dbRes : std::result::Result<u32, redis::RedisError> = redis::cmd("GET").arg("VRSC:1").query_async(&mut con).await;
//     let latest_balances: std::result::Result<TsMget<u64, f64>, redis::RedisError> = con
//         .ts_mget(
//             TsFilterOptions::default()
//                 .with_labels(true)
//                 .equals("type", "balance"),
//         )
//         .await;

//     match latest_balances {
//        Ok(res) => {
//             let mut sum : f64 = 0.0;

//             for entry in res.values.iter(){
//                 for values in entry.value{
//                     sum += values.1;
//                 }
//             }redis::cmd("FT.SEARCH")
//         Err(err) => {
//             eprintln!("err: {}", err);
//             return HttpResponse::InternalServerError().body("Data not found");
//         }
//     }
// }

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

// #[get("/minerCount")]
// async fn current_hashrate(req: HttpRequest, api_data: web::Data<SickApiData>) -> impl Responder {
//     let con = api_data.redis.clone();
//     redis::cmd("TS.GET")

//     HttpResponse::Ok().body("Hey there!")
// }
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

#[derive(Deserialize)]
struct Query {
    page: u32,
    limit: u8,
    sortby: String,
    sortdir: String,
    // chains: Vec<String>,
}

#[get("/blocks")]
async fn blocks(info: web::Query<Query>, api_data: web::Data<SickApiData>) -> impl Responder {
    // let offset : u32 = info.page * info.limit as u32;
    //TODO: check validity of sortby and sortdir and chains

    let mut sort_index = info.sortby.clone();
    sort_index.insert_str(0, "block-index:");
    // println!("{}", sort_index);

    let mut con = api_data.redis.clone();
    //TODO: match

    let mut cmd = redis::cmd("FCALL");

    cmd.arg("getblocksbyindex")
        .arg(1)
        .arg(sort_index)
        .arg(info.page * info.limit as u32) // offset
        .arg(info.page * info.limit as u32 + info.limit as u32 - 1); // num (limit)

    if info.sortdir == "desc" {
        cmd.arg("REV");
    }

    let res: TableRes<Block> = match cmd.query_async(&mut con).await {
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

    let json_res = json!(res).to_string();
    return HttpResponse::Ok().body(json_res);
}

#[get("/currentEffortPoW")]
async fn current_effort_pow(req: HttpRequest, api_data: web::Data<SickApiData>) -> impl Responder {
    let mut con = api_data.redis.clone();
    let effort_res: std::result::Result<HashMap<String, f64>, redis::RedisError> =
        redis::cmd("HGETALL")
            .arg("VRSCTEST:round_effort_pow")
            .query_async(&mut con)
            .await;

    match effort_res {
        Ok(effort_map) => {
            let mut effort: f64 = 0.0;
            match effort_map.get("total") {
                Some(total) => match effort_map.get("estimated") {
                    Some(estimated) => {
                        effort = total / estimated;
                    }
                    None => {
                        eprintln!("Estimated hashes missing!");
                    }
                },
                None => {
                    eprintln!("Total hashes missing!");
                }
            }
            return HttpResponse::Ok().body(effort.to_string());
        }
        Err(err) => {
            eprintln!("err: {}", err);
            return HttpResponse::InternalServerError().body("Data not found");
        }
    }
}

pub fn pool_route(cfg: &mut web::ServiceConfig) {
    cfg.service(staking_balance);
    // cfg.service(current_hashrate);
    cfg.service(worker_count);
    cfg.service(block_number);
    cfg.service(current_effort_pow);
    cfg.service(blocks);
    cfg.service(solvers);
    cfg.service(hashrate_history);
}

#[get("/hashrateHistory")]
async fn hashrate_history(api_data: web::Data<SickApiData>) -> impl Responder {
    let mut con = api_data.redis.clone();

    let tms: TsRange<u64, f64> = match con
        .ts_range(
            "hashrate:pool:IL",
            0,
            "+",
            None::<usize>,
            None
        )
        .await
    {
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
        res_vec.push((el.0, el.1));
    }

    HttpResponse::Ok().body(
        json!({
            "error": Value::Null,
            "result": res_vec
        })
        .to_string(),
    )
}

#[get("/solvers")]
async fn solvers(info: web::Query<Query>, api_data: web::Data<SickApiData>) -> impl Responder {
    let solver_index_prefix = "solver-index:";

    let mut con = api_data.redis.clone();
    let mut cmd = redis::cmd("FCALL");
    cmd.arg("getsolversbyindex")
        .arg(2)
        .arg(solver_index_prefix)
        .arg(info.sortby.clone())
        .arg(info.page * info.limit as u32)
        .arg((info.page + 1) * info.limit as u32 - 1);

    if info.sortdir == "desc " {
        cmd.arg("REV");
    }

    let res: TableRes<Solver> = match cmd
        .query_async(&mut con)
        .await
    {
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

    let json_res = json!(res).to_string();
    println!("{:?}", res);
    // match res{
    //     Ok(it) => {
    //         let keys : Vec<_> = it.collect();
    //         println!("{:?}", keys);
    return HttpResponse::Ok().body(json_res);
}

impl redis::FromRedisValue for Solver {
    fn from_redis_value(value: &redis::Value) -> redis::RedisResult<Self> {
        let vec: Vec<String> = redis::from_redis_value(value)?;
        Ok(Solver {
            address: vec[0].clone(),
            hashrate: vec[1].parse().unwrap_or(0f64),
            balance: vec[2].parse().unwrap_or(0f64),
            joined: vec[3].parse().unwrap(), // don't handle
            worker_count: vec[4].parse().unwrap_or(0),
        })
    }
}

// impl redis::FromRedisValue for BlockRes {
//     fn from_redis_value(value: &redis::Value) -> redis::RedisResult<Self> {
//         let search_res: SearchRes = redis::from_redis_value(value).unwrap();
//         println!("{:?}", search_res);
//         let mut blocks_vec: Vec<Block> = Vec::new();
//         // TODO: exception handle
//         for item in search_res.results {
//             // let block = parse_block(item);
//             // match block {
//             //     Ok(block) => blocks_vec.push(block),
//             //     Err(err) => eprintln!("Failed to parse block: {}", err),
//             }
//         }

//         Ok(BlockRes {
//             blocks: blocks_vec,
//             total: search_res.total,
//         })
//     }
// }

// pub enum BlockStatus {
//     Rejected = 0,
//     Accepted = 1,
// }

// pub fn parse_block(hm: HashMap<String, String>) -> Result<Block, String> {
//     let block = Block {
//         number: hm
//             .get("number")
//             .ok_or("No number key")?
//             .parse::<u32>()
//             .unwrap(),
//         height: hm
//             .get("height")
//             .ok_or("No height key")?
//             .parse::<u32>()
//             .unwrap(),
//         chain: hm.get("chain").ok_or("No chain key")?.to_string(),
//         block_type: hm.get("type").ok_or("No type key")?.to_string(),
//         is_accepted: if hm
//             .get("accepted")
//             .ok_or("No accepted key")?
//             .parse::<u8>()
//             .unwrap()
//             == 1
//         {
//             true
//         } else {
//             false
//         },
//         time: hm.get("time").ok_or("No time key")?.parse::<u64>().unwrap(),
//         duration: hm
//             .get("duration")
//             .ok_or("No duration key")?
//             .parse::<u64>()
//             .unwrap(),
//         solver: hm.get("solver").ok_or("No solver key")?.to_string(),
//         reward: hm
//             .get("reward")
//             .ok_or("No reward key")?
//             .parse::<u64>()
//             .unwrap(),
//         difficulty: hm
//             .get("difficulty")
//             .ok_or("No difficulty key")?
//             .parse::<f64>()
//             .unwrap(),
//         effort_percent: hm
//             .get("effort_percent")
//             .ok_or("No effort_percent key")?
//             .parse::<f64>()
//             .unwrap(),
//         hash: hm.get("hash").ok_or("No hash key")?.to_string(),
//     };
//     Ok(block)
// }

// impl<T: redis::FromRedisValue> redis::FromRedisValue for FtQuery<T> {
//     fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Self> {
//         let mut bulk = match v {
//             redis::Value::Bulk(v) => v.into_iter(),
//             _ => {
//                 return Err(redis::RedisError::from((
//                     redis::ErrorKind::TypeError,
//                     "Type error",
//                 )))
//             }
//         };

//         let count = match bulk.next() {
//             Some(redis::Value::Int(v)) => *v,
//             _ => {
//                 return Err(redis::RedisError::from((
//                     redis::ErrorKind::TypeError,
//                     "Type error",
//                 )))
//             }
//         };

//         let mut result = Vec::with_capacity(count.try_into().unwrap_or("missing"));

//         while let Some(v) = bulk.next() {
//             let key = <String as FromRedisValue>::from_redis_value(&v)?;
//             let value = match bulk.next() {
//                 Some(redis::Value::Bulk(v)) => T::from_redis_value(v.last().unwrap())?,
//                 _ => {
//                     return Err(redis::RedisError::from((
//                         redis::ErrorKind::TypeError,
//                         "Type error",
//                     )))
//                 }
//             };
//             result.push((key, value));
//         }

//         Ok(Self { container: result })
//     }
// }
//TODO: make general for search then specify for blocks
